mod cli;
mod error;
mod output;
mod parsers;
mod providers;
mod scorer;
mod types;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use futures::future::join_all;
use reqwest::Client;
use tokio::sync::Semaphore;
use tracing::warn;

use cli::{Cli, Commands, OutputFormat};
use output::json::JsonFormatter;
use output::table::TableFormatter;
use output::OutputFormatter;
use parsers::package_json::{get_project_name, PackageJsonParser};
use parsers::DependencyParser;
use providers::github::GitHubProvider;
use providers::npm::{extract_github_owner_repo, NpmProvider};
use providers::osv::OsvProvider;
use providers::MetadataProvider;
use providers::SignalProvider;
use scorer::HealthScorer;
use types::{DriftReport, RawSignals, ReportSummary, RiskGrade};

const MAX_CONCURRENT_REQUESTS: usize = 10;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("DRIFT_LOG"))
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Check {
            packages,
            format,
            include_dev,
            verbose,
        } => {
            run_check(packages, format, include_dev, verbose).await?;
        }
    }

    Ok(())
}

async fn run_check(
    filter_packages: Vec<String>,
    format: OutputFormat,
    include_dev: bool,
    verbose: bool,
) -> Result<()> {
    let cwd = PathBuf::from(".");

    // package.json 파싱
    let parser = PackageJsonParser;
    if !parser.detect(&cwd) {
        eprintln!(
            "{}",
            "오류: 현재 디렉토리에서 package.json을 찾을 수 없습니다.".red()
        );
        std::process::exit(1);
    }

    let mut deps = parser
        .parse(&cwd, include_dev)
        .context("package.json 파싱 실패")?;

    if deps.is_empty() {
        eprintln!("{}", "의존성이 없습니다.".yellow());
        return Ok(());
    }

    // 특정 패키지 필터링
    if !filter_packages.is_empty() {
        deps.retain(|d| filter_packages.contains(&d.name));
        if deps.is_empty() {
            eprintln!(
                "{}",
                "지정한 패키지가 dependencies에 없습니다.".yellow()
            );
            return Ok(());
        }
    }

    let project_name = get_project_name(&cwd);
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let npm_provider = Arc::new(NpmProvider::new(client.clone()));
    let github_provider = Arc::new(GitHubProvider::new(client.clone()));
    let osv_provider = Arc::new(OsvProvider::new(client.clone()));
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));

    // GITHUB_TOKEN 없으면 경고
    if std::env::var("GITHUB_TOKEN").is_err() {
        eprintln!(
            "{}",
            "⚠ GITHUB_TOKEN 미설정: rate limit 60 req/h. 설정 시 5000 req/h로 확장됩니다."
                .yellow()
        );
    }

    eprintln!(
        "{}",
        format!("🔍 {} 의존성 분석 중...", deps.len()).dimmed()
    );

    // 모든 패키지를 병렬로 분석
    let tasks: Vec<_> = deps
        .iter()
        .map(|dep| {
            let sem = semaphore.clone();
            let npm = npm_provider.clone();
            let github = github_provider.clone();
            let osv = osv_provider.clone();
            let dep_name = dep.name.clone();
            let dep_version = dep.version.clone();

            tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore acquire 실패");
                collect_and_score(&dep_name, &dep_version, &*npm, &*github, &*osv).await
            })
        })
        .collect();

    let results = join_all(tasks).await;

    let mut packages = Vec::new();
    for result in results {
        match result {
            Ok(report) => packages.push(report),
            Err(e) => warn!("패키지 분석 실패: {e}"),
        }
    }

    // 점수순 정렬 (낮은 점수 먼저)
    packages.sort_by(|a, b| {
        a.health_score
            .partial_cmp(&b.health_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let summary = ReportSummary {
        safe_count: packages
            .iter()
            .filter(|p| p.grade == RiskGrade::Safe)
            .count(),
        watch_count: packages
            .iter()
            .filter(|p| p.grade == RiskGrade::Watch)
            .count(),
        risk_count: packages
            .iter()
            .filter(|p| p.grade == RiskGrade::Risk)
            .count(),
        dead_count: packages
            .iter()
            .filter(|p| p.grade == RiskGrade::Dead)
            .count(),
        action_required: packages
            .iter()
            .filter(|p| p.grade == RiskGrade::Risk || p.grade == RiskGrade::Dead)
            .count(),
    };

    let report = DriftReport {
        project_name,
        total_deps: packages.len(),
        packages,
        summary,
    };

    // 출력
    let formatter: Box<dyn OutputFormatter> = match format {
        OutputFormat::Table => Box::new(TableFormatter),
        OutputFormat::Json => Box::new(JsonFormatter),
    };

    println!("{}", formatter.format(&report, verbose));

    // exit code: Risk/Dead 존재 시 2
    if report.summary.action_required > 0 {
        std::process::exit(2);
    }

    Ok(())
}

async fn collect_and_score(
    name: &str,
    version: &str,
    npm: &NpmProvider,
    github: &GitHubProvider,
    osv: &OsvProvider,
) -> types::PackageReport {
    let scorer = HealthScorer::new();

    // 1. npm에서 메타데이터 가져오기 (리포 URL 포함)
    let metadata = npm.get_metadata(name).await.ok();
    let repo_url = metadata.as_ref().and_then(|m| m.repository_url.as_deref());
    let is_deprecated = metadata.as_ref().is_some_and(|m| m.deprecated);

    // 2. 3개 프로바이더 병렬 수집
    let (npm_signals, github_signals, osv_signals) = tokio::join!(
        npm.collect(name, repo_url),
        github.collect(name, repo_url),
        osv.collect(name, None),
    );

    // 3. 신호 병합
    let mut raw = RawSignals::default();
    raw.is_deprecated = is_deprecated;

    if let Ok(g) = github_signals {
        raw.last_commit = g.last_commit;
        raw.release_frequency = g.release_frequency;
        raw.maintainer_count = g.maintainer_count;
        raw.issue_response_median_hours = g.issue_response_median_hours;
        raw.is_archived = g.is_archived;
        raw.pr_merge_rate = g.pr_merge_rate;
    }

    if let Ok(n) = npm_signals {
        raw.download_trend = n.download_trend;
    }

    if let Ok(o) = osv_signals {
        raw.open_cve_count = o.open_cve_count;
    }

    // star_trend는 추가 API 호출 필요 → 향후 구현
    if repo_url.is_some_and(|url| extract_github_owner_repo(url).is_some()) {
        raw.star_trend = None;
    }

    scorer.score(name, version, &raw)
}
