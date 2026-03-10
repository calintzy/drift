# Drift MVP Design Document

> **Summary**: 7개 신호 기반 의존성 건강 점수 CLI — Rust 모듈 설계 및 데이터 플로우 명세
>
> **Project**: Drift
> **Version**: 0.1.0
> **Author**: Ryan
> **Date**: 2026-03-10
> **Status**: Draft
> **Planning Doc**: [drift-mvp.plan.md](../../01-plan/features/drift-mvp.plan.md)

---

## 1. Overview

### 1.1 Design Goals

- **단일 책임**: 각 모듈이 하나의 역할만 수행 (파싱, API 호출, 점수 계산, 출력)
- **확장성**: 새로운 생태계(pip, cargo)와 신호 추가가 최소 변경으로 가능한 트레잇 기반 설계
- **내결함성**: API 실패 시 부분 점수로 graceful degradation
- **성능**: tokio 비동기 + 병렬 API 호출로 50개 의존성 30초 이내 처리

### 1.2 Design Principles

- **Trait 기반 추상화**: Provider, Parser, OutputFormatter를 트레잇으로 정의하여 구현체 교체 용이
- **Fail-Open 전략**: 개별 신호 수집 실패 시 해당 신호를 제외하고 나머지로 점수 계산
- **Zero-Config**: `GITHUB_TOKEN` 없이도 동작, 토큰 있으면 rate limit 확장

---

## 2. Architecture

### 2.1 Component Diagram

```
┌──────────────────────────────────────────────────────────────┐
│                         drift CLI                            │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────┐    ┌──────────┐    ┌─────────┐    ┌─────────┐ │
│  │   CLI   │───▶│  Parser  │───▶│ Scorer  │───▶│ Output  │ │
│  │ (clap)  │    │          │    │ Engine  │    │Formatter│ │
│  └─────────┘    └──────────┘    └────┬────┘    └─────────┘ │
│                                      │                      │
│                          ┌───────────┼───────────┐          │
│                          ▼           ▼           ▼          │
│                    ┌──────────┐ ┌─────────┐ ┌────────┐     │
│                    │ GitHub   │ │  npm    │ │  OSV   │     │
│                    │ Provider │ │Provider │ │Provider│     │
│                    └──────────┘ └─────────┘ └────────┘     │
│                          │           │           │          │
└──────────────────────────┼───────────┼───────────┼──────────┘
                           ▼           ▼           ▼
                     GitHub API   npm Registry   OSV API
```

### 2.2 Data Flow

```
1. CLI 파싱
   drift check [--format json] [packages...]
        │
2. 의존성 수집
        ▼
   Parser: package.json → Vec<Dependency>
        │
3. 패키지→리포지토리 매핑
        ▼
   npm Registry API → repository URL 추출
        │
4. 신호 수집 (병렬)
        ▼
   ┌─────────────────────────────────────────┐
   │  tokio::join! (per package, concurrent) │
   │  ├── GitHub: 커밋, 릴리스, 메인테이너,     │
   │  │          이슈 응답율, 커뮤니티          │
   │  ├── npm: 다운로드 추세                   │
   │  └── OSV: CVE 이력                       │
   └─────────────────────────────────────────┘
        │
5. 점수 계산
        ▼
   Scorer: Vec<Signal> → HealthScore (0~100)
        │
6. 등급 분류
        ▼
   HealthScore → RiskGrade (Safe/Watch/Risk/Dead)
        │
7. 출력
        ▼
   OutputFormatter: Vec<PackageReport> → Table | JSON
```

### 2.3 Dependencies (Crate)

| Crate | Version | Purpose |
|-------|---------|---------|
| `clap` | 4.x | CLI 인자 파싱 (derive 매크로) |
| `tokio` | 1.x | 비동기 런타임 (rt-multi-thread, macros) |
| `reqwest` | 0.12.x | HTTP 클라이언트 (json, rustls-tls) |
| `serde` | 1.x | 직렬화/역직렬화 (derive) |
| `serde_json` | 1.x | JSON 파싱/출력 |
| `comfy-table` | 7.x | 터미널 테이블 렌더링 |
| `colored` | 2.x | 터미널 컬러 출력 |
| `thiserror` | 1.x | 커스텀 에러 타입 |
| `anyhow` | 1.x | main 함수 에러 핸들링 |
| `tracing` | 0.1.x | 구조적 로깅 |
| `tracing-subscriber` | 0.3.x | 로그 출력 설정 |
| `chrono` | 0.4.x | 날짜/시간 계산 |
| `futures` | 0.3.x | 비동기 유틸리티 (join_all) |

**Dev Dependencies:**

| Crate | Purpose |
|-------|---------|
| `wiremock` | HTTP API 모킹 |
| `tokio-test` | 비동기 테스트 유틸리티 |
| `assert_cmd` | CLI 통합 테스트 |
| `predicates` | 출력 검증 |
| `tempfile` | 임시 파일/디렉토리 |

---

## 3. Data Model

### 3.1 Core Types (`src/types.rs`)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// package.json에서 파싱한 의존성 정보
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub dep_type: DepType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DepType {
    Production,
    Development,
}

/// npm registry에서 가져온 패키지 메타데이터
#[derive(Debug, Clone)]
pub struct PackageMetadata {
    pub name: String,
    pub repository_url: Option<String>,
    pub latest_version: String,
}

/// 7개 신호의 원시 데이터
#[derive(Debug, Clone, Default)]
pub struct RawSignals {
    pub last_commit: Option<DateTime<Utc>>,
    pub release_frequency: Option<f64>,      // 월간 릴리스 횟수
    pub maintainer_count: Option<u32>,
    pub issue_response_median_hours: Option<f64>,
    pub download_trend: Option<f64>,         // -1.0 ~ 1.0 (하락~상승)
    pub open_cve_count: Option<u32>,
    pub cve_patch_speed_days: Option<f64>,
    pub star_trend: Option<f64>,             // -1.0 ~ 1.0
    pub pr_merge_rate: Option<f64>,          // 0.0 ~ 1.0
    pub is_archived: bool,
    pub is_deprecated: bool,
}

/// 개별 신호 점수 (0~100)
#[derive(Debug, Clone, Serialize)]
pub struct SignalScore {
    pub name: &'static str,
    pub weight: f64,
    pub score: f64,         // 0.0 ~ 100.0
    pub available: bool,    // 데이터 수집 성공 여부
    pub detail: String,     // 사람이 읽을 수 있는 설명
}

/// 패키지별 최종 건강 보고서
#[derive(Debug, Clone, Serialize)]
pub struct PackageReport {
    pub name: String,
    pub version: String,
    pub health_score: f64,           // 0.0 ~ 100.0
    pub grade: RiskGrade,
    pub signal_scores: Vec<SignalScore>,
    pub summary_signal: String,      // 핵심 신호 요약 텍스트
}

/// 리스크 등급
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum RiskGrade {
    Safe,   // 80~100
    Watch,  // 60~79
    Risk,   // 40~59
    Dead,   // 0~39
}

impl RiskGrade {
    pub fn from_score(score: f64) -> Self {
        match score as u32 {
            80..=100 => Self::Safe,
            60..=79 => Self::Watch,
            40..=59 => Self::Risk,
            _ => Self::Dead,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Watch => "Watch",
            Self::Risk => "Risk",
            Self::Dead => "Dead",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Safe => "🟢",
            Self::Watch => "🟡",
            Self::Risk => "🟠",
            Self::Dead => "🔴",
        }
    }
}

/// 전체 스캔 결과
#[derive(Debug, Serialize)]
pub struct DriftReport {
    pub project_name: String,
    pub total_deps: usize,
    pub packages: Vec<PackageReport>,
    pub summary: ReportSummary,
}

#[derive(Debug, Serialize)]
pub struct ReportSummary {
    pub safe_count: usize,
    pub watch_count: usize,
    pub risk_count: usize,
    pub dead_count: usize,
    pub action_required: usize,
}
```

---

## 4. Module Specification

### 4.1 CLI Module (`src/cli.rs`)

```rust
use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(name = "drift", version, about = "Dependency health monitor")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    /// 의존성 건강 점수 체크
    Check {
        /// 특정 패키지만 체크 (생략 시 전체)
        packages: Vec<String>,

        /// 출력 포맷
        #[arg(long, default_value = "table")]
        format: OutputFormat,

        /// devDependencies 포함 여부
        #[arg(long, default_value = "false")]
        include_dev: bool,

        /// 상세 신호 점수 표시
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(ValueEnum, Clone)]
pub enum OutputFormat {
    Table,
    Json,
}
```

**CLI 사용 예시:**
```bash
drift check                          # 전체 스캔 (production deps)
drift check --include-dev            # devDependencies 포함
drift check axios lodash             # 특정 패키지만
drift check --format json            # JSON 출력
drift check --verbose                # 개별 신호 점수 표시
```

### 4.2 Parser Module (`src/parsers/`)

**트레잇 정의 (`mod.rs`):**
```rust
use crate::types::Dependency;
use anyhow::Result;

pub trait DependencyParser {
    fn parse(&self, path: &std::path::Path) -> Result<Vec<Dependency>>;
    fn detect(&self, path: &std::path::Path) -> bool;
}
```

**package.json 파서 (`package_json.rs`):**

| 메서드 | 입력 | 출력 | 설명 |
|--------|------|------|------|
| `parse()` | `Path` (package.json 경로) | `Vec<Dependency>` | dependencies + devDependencies 파싱 |
| `detect()` | `Path` (프로젝트 루트) | `bool` | package.json 존재 여부 |

**파싱 로직:**
1. `package.json` 읽기 → `serde_json::Value`로 파싱
2. `dependencies` 객체에서 `name: version` 추출 → `DepType::Production`
3. `--include-dev`이면 `devDependencies`도 추출 → `DepType::Development`
4. workspace 패키지(`workspace:*`) 제외

### 4.3 Provider Module (`src/providers/`)

**트레잇 정의 (`mod.rs`):**
```rust
use crate::types::RawSignals;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait SignalProvider {
    async fn collect(&self, package_name: &str, repo_url: Option<&str>) -> Result<RawSignals>;
}
```

#### 4.3.1 GitHub Provider (`github.rs`)

| API Endpoint | 수집 데이터 | 매핑 신호 |
|-------------|------------|----------|
| `GET /repos/{owner}/{repo}` | archived, stargazers_count, updated_at | is_archived, star_trend |
| `GET /repos/{owner}/{repo}/commits?per_page=1` | 최신 커밋 날짜 | last_commit |
| `GET /repos/{owner}/{repo}/releases?per_page=10` | 릴리스 날짜 목록 | release_frequency |
| `GET /repos/{owner}/{repo}/contributors?per_page=100` | 컨트리뷰터 수 | maintainer_count |
| `GET /repos/{owner}/{repo}/issues?state=closed&per_page=30` | 이슈 생성→첫 응답 시간 | issue_response_median_hours |
| `GET /repos/{owner}/{repo}/pulls?state=all&per_page=30` | 머지된 PR / 전체 PR | pr_merge_rate |

**Rate Limit 대응:**
- `GITHUB_TOKEN` 환경변수 → `Authorization: Bearer {token}` 헤더
- 비인증: 60 req/h, 인증: 5000 req/h
- `X-RateLimit-Remaining` 헤더 모니터링
- 0에 도달 시 경고 메시지 출력 후 나머지 신호로 부분 점수 계산

**Repository URL 매핑:**
1. npm Registry API 응답의 `repository.url` 필드 추출
2. `git+https://github.com/user/repo.git` → `user/repo`로 정규화
3. `github:user/repo` 단축 형식 지원
4. 매핑 실패 시 GitHub 관련 신호 모두 `available: false`

#### 4.3.2 npm Provider (`npm.rs`)

| API Endpoint | 수집 데이터 | 매핑 신호 |
|-------------|------------|----------|
| `GET https://registry.npmjs.org/{package}` | repository, deprecated, latest version | repository_url, is_deprecated |
| `GET https://api.npmjs.org/downloads/range/last-month/{package}` | 일별 다운로드 | download_trend (최근 vs 이전) |

**다운로드 추세 계산:**
```
trend = (최근 2주 평균 - 이전 2주 평균) / 이전 2주 평균
결과: -1.0(급감) ~ 0.0(동일) ~ 1.0+(급증)
```

#### 4.3.3 OSV Provider (`osv.rs`)

| API Endpoint | 수집 데이터 | 매핑 신호 |
|-------------|------------|----------|
| `POST https://api.osv.dev/v1/query` | 알려진 취약점 목록 | open_cve_count |

**요청 본문:**
```json
{
  "package": {
    "name": "{package_name}",
    "ecosystem": "npm"
  }
}
```

### 4.4 Scorer Module (`src/scorer/`)

#### 신호 점수 계산 (`signals.rs`)

각 신호를 0~100 점수로 정규화:

| Signal | 계산 공식 | 비고 |
|--------|----------|------|
| **last_commit** | `max(0, 100 - (days_since * 100 / 365))` | archived면 0점 |
| **release_frequency** | `min(100, monthly_releases * 50)` | 월 2회 이상이면 100 |
| **maintainer_count** | `min(100, count * 20)` | 5명 이상이면 100 |
| **issue_response** | `max(0, 100 - (median_hours * 100 / 720))` | 30일(720h) 이상이면 0 |
| **download_trend** | `(trend + 1.0) * 50` | -1.0→0, 0.0→50, 1.0→100 |
| **cve_history** | `max(0, 100 - (open_cves * 33))` | 3개 이상 미패치면 0 |
| **community** | `(star_trend_score + pr_merge_score) / 2` | 각각 0~100 |

#### 최종 점수 계산 (`model.rs`)

```rust
pub struct ScoringModel {
    pub weights: [(& 'static str, f64); 7],
}

impl Default for ScoringModel {
    fn default() -> Self {
        Self {
            weights: [
                ("last_commit",       0.20),
                ("release_frequency", 0.15),
                ("maintainer_count",  0.15),
                ("issue_response",    0.15),
                ("download_trend",    0.15),
                ("cve_history",       0.10),
                ("community",         0.10),
            ],
        }
    }
}
```

**가중치 재정규화 (Fail-Open):**
- 7개 중 일부 신호 수집 실패 시, 수집 성공한 신호의 가중치 합이 1.0이 되도록 재정규화
- 예: GitHub API 실패로 5개 신호 누락 → npm(0.15) + CVE(0.10) 가중치를 0.60 + 0.40으로 재정규화
- 수집된 신호가 2개 미만이면 점수 대신 "Insufficient Data" 표시

### 4.5 Output Module (`src/output/`)

#### 터미널 테이블 (`table.rs`)

```
📊 Dependency Health Report for my-project
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Package           Health  Risk     Signal
─────────────────────────────────────────
react             98/100  🟢 Safe  Active, 1,200+ contributors
axios             72/100  🟡 Watch Fewer maintainers, slower releases
moment            23/100  🔴 Dead  Deprecated, use dayjs/date-fns

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Summary: 47 deps | 38 safe | 6 watch | 3 risk
Action Required: 2 critical replacements suggested
```

**`--verbose` 모드 추가 출력:**
```
  axios (72/100 🟡 Watch)
  ├── Last Commit:      85/100 (14 days ago)
  ├── Release Freq:     60/100 (0.8/month)
  ├── Maintainers:      40/100 (2 active)
  ├── Issue Response:   70/100 (median 48h)
  ├── Download Trend:   90/100 (+12% monthly)
  ├── CVE History:     100/100 (no open CVEs)
  └── Community:        65/100 (stars stable)
```

#### JSON 출력 (`json.rs`)

`DriftReport` 구조체를 `serde_json::to_string_pretty()`로 직렬화.

---

## 5. Error Handling

### 5.1 에러 타입 정의 (`src/error.rs`)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DriftError {
    #[error("package.json을 찾을 수 없습니다: {path}")]
    PackageJsonNotFound { path: String },

    #[error("package.json 파싱 실패: {reason}")]
    ParseError { reason: String },

    #[error("GitHub API 오류: {status} - {message}")]
    GitHubApiError { status: u16, message: String },

    #[error("GitHub API rate limit 초과 (리셋: {reset_at})")]
    RateLimitExceeded { reset_at: String },

    #[error("npm Registry API 오류: {message}")]
    NpmApiError { message: String },

    #[error("OSV API 오류: {message}")]
    OsvApiError { message: String },

    #[error("네트워크 오류: {source}")]
    NetworkError { source: reqwest::Error },

    #[error("리포지토리 URL을 확인할 수 없습니다: {package}")]
    RepoNotFound { package: String },
}
```

### 5.2 에러 처리 전략

| 에러 유형 | 처리 방식 | 사용자 영향 |
|----------|----------|------------|
| package.json 없음 | 즉시 종료 + 에러 메시지 | exit code 1 |
| GitHub rate limit | 경고 출력 + GitHub 신호 제외 | 부분 점수 (⚠ 표시) |
| 개별 API 실패 | 해당 신호 `available: false` | 부분 점수 + 경고 |
| 네트워크 전체 실패 | 전체 실패 + 에러 메시지 | exit code 1 |
| 리포 매핑 실패 | GitHub 신호 제외 | npm+OSV 신호만 사용 |

### 5.3 Exit Codes

| Code | 의미 |
|------|------|
| 0 | 성공 (모든 의존성 Safe 또는 Watch) |
| 1 | 오류 (파싱 실패, 네트워크 전체 실패 등) |
| 2 | 경고 (Risk 또는 Dead 등급 의존성 존재) — CI 통합용 |

---

## 6. Concurrency Design

### 6.1 병렬화 전략

```rust
// 의존성별 병렬 처리 (최대 동시 10개)
let semaphore = Arc::new(Semaphore::new(10));

let tasks: Vec<_> = dependencies.iter().map(|dep| {
    let sem = semaphore.clone();
    let providers = providers.clone();
    tokio::spawn(async move {
        let _permit = sem.acquire().await.unwrap();
        collect_and_score(dep, &providers).await
    })
}).collect();

let results = futures::future::join_all(tasks).await;
```

### 6.2 패키지 내 신호 수집 병렬화

```rust
async fn collect_signals(pkg: &str, repo: Option<&str>) -> RawSignals {
    let (github_result, npm_result, osv_result) = tokio::join!(
        github_provider.collect(pkg, repo),
        npm_provider.collect(pkg, None),
        osv_provider.collect(pkg, None),
    );
    merge_signals(github_result, npm_result, osv_result)
}
```

---

## 7. Test Plan

### 7.1 Test Scope

| Type | Target | Tool | Coverage |
|------|--------|------|----------|
| Unit Test | 점수 계산 로직, 등급 분류 | `cargo test` | 각 신호 계산, 경계값 |
| Unit Test | package.json 파싱 | `cargo test` | 다양한 포맷, 엣지 케이스 |
| Integration Test | API 호출 + 점수 계산 파이프라인 | `wiremock` | 모킹된 API 응답 |
| Integration Test | CLI 전체 실행 | `assert_cmd` | 인자 조합, 출력 포맷 |
| Snapshot Test | 테이블/JSON 출력 형식 | `insta` (선택) | 출력 회귀 방지 |

### 7.2 Test Cases (Key)

**Unit Tests:**
- [ ] `RiskGrade::from_score(80)` → `Safe`, `from_score(79)` → `Watch`
- [ ] 모든 신호 100점일 때 최종 점수 100
- [ ] 신호 3개만 수집 시 가중치 재정규화 검증
- [ ] deprecated 패키지 → 강제 Dead 등급
- [ ] archived 리포 → last_commit 0점
- [ ] package.json에 dependencies 없을 때 빈 목록 반환
- [ ] workspace 프로토콜(`workspace:*`) 필터링

**Integration Tests:**
- [ ] 정상 API 응답 → 올바른 점수 및 테이블 출력
- [ ] GitHub API 429 → 경고 메시지 + 부분 점수
- [ ] `--format json` → 유효한 JSON 출력
- [ ] `drift check react axios` → 2개 패키지만 체크
- [ ] package.json 없는 디렉토리 → exit code 1

---

## 8. File Structure

```
drift/
├── Cargo.toml
├── src/
│   ├── main.rs                 # 진입점: CLI 파싱 → 실행 흐름 조립
│   ├── cli.rs                  # clap derive 기반 CLI 정의
│   ├── error.rs                # DriftError 정의 (thiserror)
│   ├── types.rs                # 공통 타입: Dependency, RawSignals, PackageReport 등
│   ├── scorer/
│   │   ├── mod.rs              # HealthScorer 구현: 신호 → 점수 → 등급
│   │   ├── model.rs            # ScoringModel: 가중치 정의 + 재정규화
│   │   └── signals.rs          # 개별 신호 점수 계산 함수들
│   ├── providers/
│   │   ├── mod.rs              # SignalProvider 트레잇 정의
│   │   ├── github.rs           # GitHub REST API 클라이언트
│   │   ├── npm.rs              # npm Registry API 클라이언트
│   │   └── osv.rs              # OSV API 클라이언트 (CVE 조회)
│   ├── parsers/
│   │   ├── mod.rs              # DependencyParser 트레잇 정의
│   │   └── package_json.rs     # package.json 파서
│   └── output/
│       ├── mod.rs              # OutputFormatter 트레잇 정의
│       ├── table.rs            # comfy-table 터미널 출력
│       └── json.rs             # serde_json JSON 출력
└── tests/
    ├── fixtures/
    │   ├── package.json         # 정상 테스트용
    │   ├── package_empty.json   # 의존성 없는 케이스
    │   └── package_workspace.json # workspace 프로토콜 포함
    └── integration/
        ├── cli_test.rs          # assert_cmd 기반 CLI 테스트
        └── scoring_test.rs      # wiremock 기반 점수 계산 테스트
```

---

## 9. Implementation Order

| Step | Module | 파일 | 의존성 | 예상 시간 |
|------|--------|------|--------|----------|
| 1 | 프로젝트 초기화 | `Cargo.toml`, `main.rs` | 없음 | 0.5일 |
| 2 | 타입 정의 | `types.rs`, `error.rs` | 없음 | 0.5일 |
| 3 | CLI 정의 | `cli.rs` | Step 2 | 0.5일 |
| 4 | package.json 파서 | `parsers/` | Step 2 | 1일 |
| 5 | npm Provider | `providers/npm.rs` | Step 2 | 1일 |
| 6 | GitHub Provider | `providers/github.rs` | Step 2 | 2일 |
| 7 | OSV Provider | `providers/osv.rs` | Step 2 | 0.5일 |
| 8 | Scorer 엔진 | `scorer/` | Step 2 | 1.5일 |
| 9 | Table 출력 | `output/table.rs` | Step 8 | 1일 |
| 10 | JSON 출력 | `output/json.rs` | Step 8 | 0.5일 |
| 11 | main.rs 통합 | `main.rs` | Step 3~10 | 1일 |
| 12 | 통합 테스트 | `tests/` | Step 11 | 1.5일 |

**Critical Path:** Step 1→2→4→5→6→8→11→12

---

## 10. Coding Conventions

### 10.1 Naming Conventions (Rust)

| Target | Rule | Example |
|--------|------|---------|
| 함수/메서드 | snake_case | `calculate_score()`, `parse_package_json()` |
| 구조체/열거형 | PascalCase | `PackageReport`, `RiskGrade` |
| 상수 | UPPER_SNAKE_CASE | `MAX_CONCURRENT_REQUESTS`, `DEFAULT_TIMEOUT` |
| 모듈/파일 | snake_case | `package_json.rs`, `github.rs` |
| 트레잇 | PascalCase (동사/형용사) | `SignalProvider`, `DependencyParser` |

### 10.2 Import Order

```rust
// 1. std
use std::path::Path;
use std::sync::Arc;

// 2. external crates
use anyhow::Result;
use clap::Parser;
use tokio::sync::Semaphore;

// 3. crate modules
use crate::types::{Dependency, PackageReport};
use crate::providers::SignalProvider;

// 4. self/super
use super::model::ScoringModel;
```

### 10.3 Error Handling Convention

- **라이브러리 코드** (`providers/`, `scorer/`, `parsers/`): `thiserror`로 구체적 에러 타입 반환
- **진입점** (`main.rs`): `anyhow::Result`로 에러 통합 처리
- **절대 `unwrap()` 사용 금지** — `expect("reason")` 또는 `?` 사용

---

## Version History

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 0.1 | 2026-03-10 | Initial design based on drift-mvp.plan.md | Ryan |
