use crate::types::SignalScore;

/// 점수 가중치 모델
pub struct ScoringModel {
    pub weights: [(&'static str, f64); 7],
}

impl Default for ScoringModel {
    fn default() -> Self {
        Self {
            weights: [
                ("last_commit", 0.20),
                ("release_frequency", 0.15),
                ("maintainer_count", 0.15),
                ("issue_response", 0.15),
                ("download_trend", 0.15),
                ("cve_history", 0.10),
                ("community", 0.10),
            ],
        }
    }
}

impl ScoringModel {
    /// 가용한 신호만으로 가중치를 재정규화하여 최종 점수 계산
    pub fn calculate(&self, scores: &[SignalScore]) -> f64 {
        let available_weight: f64 = scores
            .iter()
            .filter(|s| s.available)
            .map(|s| s.weight)
            .sum();

        if available_weight == 0.0 {
            return 0.0;
        }

        let weighted_sum: f64 = scores
            .iter()
            .filter(|s| s.available)
            .map(|s| s.score * s.weight)
            .sum();

        // 재정규화: 가용 가중치 합으로 나누어 0~100 범위 유지
        (weighted_sum / available_weight).clamp(0.0, 100.0)
    }

    /// 수집된 신호가 충분한지 확인 (최소 2개)
    pub fn has_sufficient_data(scores: &[SignalScore]) -> bool {
        scores.iter().filter(|s| s.available).count() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_score(name: &'static str, weight: f64, score: f64, available: bool) -> SignalScore {
        SignalScore {
            name,
            weight,
            score,
            available,
            detail: String::new(),
        }
    }

    #[test]
    fn test_all_signals_perfect() {
        let model = ScoringModel::default();
        let scores = vec![
            make_score("last_commit", 0.20, 100.0, true),
            make_score("release_frequency", 0.15, 100.0, true),
            make_score("maintainer_count", 0.15, 100.0, true),
            make_score("issue_response", 0.15, 100.0, true),
            make_score("download_trend", 0.15, 100.0, true),
            make_score("cve_history", 0.10, 100.0, true),
            make_score("community", 0.10, 100.0, true),
        ];
        assert_eq!(model.calculate(&scores), 100.0);
    }

    #[test]
    fn test_partial_signals_renormalization() {
        let model = ScoringModel::default();
        // npm(0.15) + cve(0.10) 만 가용 → 재정규화
        let scores = vec![
            make_score("last_commit", 0.20, 0.0, false),
            make_score("release_frequency", 0.15, 0.0, false),
            make_score("maintainer_count", 0.15, 0.0, false),
            make_score("issue_response", 0.15, 0.0, false),
            make_score("download_trend", 0.15, 80.0, true),
            make_score("cve_history", 0.10, 100.0, true),
            make_score("community", 0.10, 0.0, false),
        ];
        // (80*0.15 + 100*0.10) / (0.15 + 0.10) = (12 + 10) / 0.25 = 88.0
        assert!((model.calculate(&scores) - 88.0).abs() < 0.1);
    }

    #[test]
    fn test_no_available_signals() {
        let model = ScoringModel::default();
        let scores = vec![make_score("last_commit", 0.20, 50.0, false)];
        assert_eq!(model.calculate(&scores), 0.0);
    }

    #[test]
    fn test_has_sufficient_data() {
        let one = vec![make_score("a", 0.5, 50.0, true)];
        let two = vec![
            make_score("a", 0.5, 50.0, true),
            make_score("b", 0.5, 50.0, true),
        ];
        assert!(!ScoringModel::has_sufficient_data(&one));
        assert!(ScoringModel::has_sufficient_data(&two));
    }
}
