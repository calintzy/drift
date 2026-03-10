use colored::Colorize;
use comfy_table::{Cell, CellAlignment, ContentArrangement, Table};

use crate::types::{DriftReport, PackageReport, RiskGrade};

use super::OutputFormatter;

pub struct TableFormatter;

impl OutputFormatter for TableFormatter {
    fn format(&self, report: &DriftReport, verbose: bool) -> String {
        let mut output = String::new();

        // 헤더
        output.push_str(&format!(
            "\n{}\n",
            format!("📊 Dependency Health Report for {}", report.project_name).bold()
        ));
        output.push_str(&"━".repeat(50));
        output.push('\n');

        // 메인 테이블
        let mut table = Table::new();
        table
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Package").set_alignment(CellAlignment::Left),
                Cell::new("Health").set_alignment(CellAlignment::Right),
                Cell::new("Risk").set_alignment(CellAlignment::Center),
                Cell::new("Signal").set_alignment(CellAlignment::Left),
            ]);

        for pkg in &report.packages {
            table.add_row(vec![
                Cell::new(&pkg.name),
                Cell::new(format!("{:.0}/100", pkg.health_score)),
                Cell::new(format!("{} {}", pkg.grade.emoji(), pkg.grade.label())),
                Cell::new(&pkg.summary_signal),
            ]);
        }

        output.push_str(&table.to_string());
        output.push('\n');

        // verbose 모드: 각 패키지의 상세 신호 점수
        if verbose {
            output.push('\n');
            for pkg in &report.packages {
                output.push_str(&format_verbose_package(pkg));
            }
        }

        // Summary
        output.push_str(&"━".repeat(50));
        output.push('\n');
        output.push_str(&format!(
            "Summary: {} deps | {} safe | {} watch | {} risk | {} dead\n",
            report.total_deps,
            report.summary.safe_count.to_string().green(),
            report.summary.watch_count.to_string().yellow(),
            report.summary.risk_count.to_string().truecolor(255, 165, 0),
            report.summary.dead_count.to_string().red(),
        ));

        if report.summary.action_required > 0 {
            output.push_str(&format!(
                "{}\n",
                format!(
                    "Action Required: {} critical replacements suggested",
                    report.summary.action_required
                )
                .red()
                .bold()
            ));
        }

        output
    }
}

fn format_verbose_package(pkg: &PackageReport) -> String {
    let mut output = String::new();
    let grade_colored = match pkg.grade {
        RiskGrade::Safe => format!("{:.0}/100 🟢 {}", pkg.health_score, pkg.grade.label())
            .green()
            .to_string(),
        RiskGrade::Watch => format!("{:.0}/100 🟡 {}", pkg.health_score, pkg.grade.label())
            .yellow()
            .to_string(),
        RiskGrade::Risk => format!("{:.0}/100 🟠 {}", pkg.health_score, pkg.grade.label())
            .truecolor(255, 165, 0)
            .to_string(),
        RiskGrade::Dead => format!("{:.0}/100 🔴 {}", pkg.health_score, pkg.grade.label())
            .red()
            .to_string(),
    };

    output.push_str(&format!("  {} ({})\n", pkg.name.bold(), grade_colored));

    let total = pkg.signal_scores.len();
    for (i, signal) in pkg.signal_scores.iter().enumerate() {
        let prefix = if i == total - 1 {
            "└──"
        } else {
            "├──"
        };
        let status = if signal.available {
            format!("{:>5.0}/100", signal.score)
        } else {
            "  N/A".to_string()
        };
        output.push_str(&format!(
            "  {} {:<20} {} ({})\n",
            prefix, signal.name, status, signal.detail
        ));
    }
    output.push('\n');
    output
}
