pub mod model;
pub mod signals;

use crate::types::{PackageReport, RawSignals, RiskGrade, SignalScore};
use model::ScoringModel;

/// 건강 점수 엔진
pub struct HealthScorer {
    model: ScoringModel,
}

impl HealthScorer {
    pub fn new() -> Self {
        Self {
            model: ScoringModel::default(),
        }
    }

    /// 원시 신호 → 패키지 보고서
    pub fn score(&self, name: &str, version: &str, raw: &RawSignals) -> PackageReport {
        let signal_scores = vec![
            signals::score_last_commit(raw),
            signals::score_release_frequency(raw),
            signals::score_maintainer_count(raw),
            signals::score_issue_response(raw),
            signals::score_download_trend(raw),
            signals::score_cve_history(raw),
            signals::score_community(raw),
        ];

        let health_score = if raw.is_deprecated {
            // deprecated 패키지는 강제 Dead
            0.0
        } else if !ScoringModel::has_sufficient_data(&signal_scores) {
            0.0
        } else {
            self.model.calculate(&signal_scores)
        };

        let grade = if raw.is_deprecated {
            RiskGrade::Dead
        } else if !ScoringModel::has_sufficient_data(&signal_scores) {
            RiskGrade::Dead
        } else {
            RiskGrade::from_score(health_score)
        };

        let summary_signal = build_summary(&signal_scores, raw);

        PackageReport {
            name: name.to_string(),
            version: version.to_string(),
            health_score,
            grade,
            signal_scores,
            summary_signal,
        }
    }
}

/// 핵심 신호를 사람이 읽을 수 있는 한 줄 요약으로 생성
fn build_summary(scores: &[SignalScore], raw: &RawSignals) -> String {
    if raw.is_deprecated {
        return "Deprecated".to_string();
    }
    if raw.is_archived {
        return "Archived repository".to_string();
    }

    let mut parts = Vec::new();

    // 가장 낮은 점수 신호를 주요 이슈로 표시
    if let Some(worst) = scores.iter().filter(|s| s.available).min_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) {
        if worst.score < 50.0 {
            parts.push(format!("{}: {}", worst.name, worst.detail));
        }
    }

    // 가장 높은 점수 신호를 강점으로 표시
    if let Some(best) = scores.iter().filter(|s| s.available).max_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) {
        if best.score >= 80.0 && parts.len() < 2 {
            parts.push(format!("{}: {}", best.name, best.detail));
        }
    }

    if parts.is_empty() {
        "Active".to_string()
    } else {
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_deprecated() {
        let scorer = HealthScorer::new();
        let raw = RawSignals {
            is_deprecated: true,
            ..Default::default()
        };
        let report = scorer.score("moment", "2.29.4", &raw);
        assert_eq!(report.health_score, 0.0);
        assert_eq!(report.grade, RiskGrade::Dead);
        assert_eq!(report.summary_signal, "Deprecated");
    }

    #[test]
    fn test_score_healthy_package() {
        let scorer = HealthScorer::new();
        let raw = RawSignals {
            last_commit: Some(chrono::Utc::now()),
            release_frequency: Some(2.0),
            maintainer_count: Some(10),
            issue_response_median_hours: Some(12.0),
            download_trend: Some(0.3),
            open_cve_count: Some(0),
            pr_merge_rate: Some(0.8),
            ..Default::default()
        };
        let report = scorer.score("react", "18.0.0", &raw);
        assert!(report.health_score > 80.0);
        assert_eq!(report.grade, RiskGrade::Safe);
    }
}
