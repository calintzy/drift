use chrono::Utc;

use crate::types::{RawSignals, SignalScore};

/// 마지막 커밋 신호 점수 계산
pub fn score_last_commit(signals: &RawSignals) -> SignalScore {
    if signals.is_archived {
        return SignalScore {
            name: "last_commit",
            weight: 0.20,
            score: 0.0,
            available: true,
            detail: "Archived repository".to_string(),
        };
    }

    match signals.last_commit {
        Some(date) => {
            let days = (Utc::now() - date).num_days().max(0) as f64;
            let score = (100.0 - (days * 100.0 / 365.0)).clamp(0.0, 100.0);
            let detail = if days < 1.0 {
                "today".to_string()
            } else {
                format!("{} days ago", days as u32)
            };
            SignalScore {
                name: "last_commit",
                weight: 0.20,
                score,
                available: true,
                detail,
            }
        }
        None => SignalScore {
            name: "last_commit",
            weight: 0.20,
            score: 0.0,
            available: false,
            detail: "N/A".to_string(),
        },
    }
}

/// 릴리스 주기 신호 점수 계산
pub fn score_release_frequency(signals: &RawSignals) -> SignalScore {
    match signals.release_frequency {
        Some(freq) => {
            let score = (freq * 50.0).clamp(0.0, 100.0);
            let detail = format!("{:.1}/month", freq);
            SignalScore {
                name: "release_frequency",
                weight: 0.15,
                score,
                available: true,
                detail,
            }
        }
        None => SignalScore {
            name: "release_frequency",
            weight: 0.15,
            score: 0.0,
            available: false,
            detail: "N/A".to_string(),
        },
    }
}

/// 메인테이너 수 신호 점수 계산
pub fn score_maintainer_count(signals: &RawSignals) -> SignalScore {
    match signals.maintainer_count {
        Some(count) => {
            let score = (count as f64 * 20.0).clamp(0.0, 100.0);
            let detail = format!("{} active", count);
            SignalScore {
                name: "maintainer_count",
                weight: 0.15,
                score,
                available: true,
                detail,
            }
        }
        None => SignalScore {
            name: "maintainer_count",
            weight: 0.15,
            score: 0.0,
            available: false,
            detail: "N/A".to_string(),
        },
    }
}

/// 이슈 응답율 신호 점수 계산
pub fn score_issue_response(signals: &RawSignals) -> SignalScore {
    match signals.issue_response_median_hours {
        Some(hours) => {
            let score = (100.0 - (hours * 100.0 / 720.0)).clamp(0.0, 100.0);
            let detail = if hours < 24.0 {
                format!("median {:.0}h", hours)
            } else {
                format!("median {:.0}d", hours / 24.0)
            };
            SignalScore {
                name: "issue_response",
                weight: 0.15,
                score,
                available: true,
                detail,
            }
        }
        None => SignalScore {
            name: "issue_response",
            weight: 0.15,
            score: 0.0,
            available: false,
            detail: "N/A".to_string(),
        },
    }
}

/// 다운로드 추세 신호 점수 계산
pub fn score_download_trend(signals: &RawSignals) -> SignalScore {
    match signals.download_trend {
        Some(trend) => {
            let score = ((trend + 1.0) * 50.0).clamp(0.0, 100.0);
            let detail = if trend > 0.0 {
                format!("+{:.0}%", trend * 100.0)
            } else {
                format!("{:.0}%", trend * 100.0)
            };
            SignalScore {
                name: "download_trend",
                weight: 0.15,
                score,
                available: true,
                detail,
            }
        }
        None => SignalScore {
            name: "download_trend",
            weight: 0.15,
            score: 0.0,
            available: false,
            detail: "N/A".to_string(),
        },
    }
}

/// CVE 이력 신호 점수 계산
pub fn score_cve_history(signals: &RawSignals) -> SignalScore {
    match signals.open_cve_count {
        Some(count) => {
            let score = (100.0 - (count as f64 * 33.0)).clamp(0.0, 100.0);
            let detail = if count == 0 {
                "no open CVEs".to_string()
            } else {
                format!("{} open CVEs", count)
            };
            SignalScore {
                name: "cve_history",
                weight: 0.10,
                score,
                available: true,
                detail,
            }
        }
        None => SignalScore {
            name: "cve_history",
            weight: 0.10,
            score: 0.0,
            available: false,
            detail: "N/A".to_string(),
        },
    }
}

/// 커뮤니티 신호 점수 계산
pub fn score_community(signals: &RawSignals) -> SignalScore {
    let star_score = signals
        .star_trend
        .map(|t| ((t + 1.0) * 50.0).clamp(0.0, 100.0));
    let pr_score = signals
        .pr_merge_rate
        .map(|r| (r * 100.0).clamp(0.0, 100.0));

    match (star_score, pr_score) {
        (Some(s), Some(p)) => SignalScore {
            name: "community",
            weight: 0.10,
            score: (s + p) / 2.0,
            available: true,
            detail: format!("PR merge {:.0}%", pr_score.unwrap_or(0.0)),
        },
        (None, Some(p)) => SignalScore {
            name: "community",
            weight: 0.10,
            score: p,
            available: true,
            detail: format!("PR merge {:.0}%", p),
        },
        (Some(s), None) => SignalScore {
            name: "community",
            weight: 0.10,
            score: s,
            available: true,
            detail: "stars only".to_string(),
        },
        (None, None) => SignalScore {
            name: "community",
            weight: 0.10,
            score: 0.0,
            available: false,
            detail: "N/A".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_score_last_commit_recent() {
        let signals = RawSignals {
            last_commit: Some(Utc::now() - Duration::days(7)),
            ..Default::default()
        };
        let score = score_last_commit(&signals);
        assert!(score.score > 95.0);
        assert!(score.available);
    }

    #[test]
    fn test_score_last_commit_old() {
        let signals = RawSignals {
            last_commit: Some(Utc::now() - Duration::days(400)),
            ..Default::default()
        };
        let score = score_last_commit(&signals);
        assert_eq!(score.score, 0.0);
    }

    #[test]
    fn test_score_last_commit_archived() {
        let signals = RawSignals {
            last_commit: Some(Utc::now()),
            is_archived: true,
            ..Default::default()
        };
        let score = score_last_commit(&signals);
        assert_eq!(score.score, 0.0);
    }

    #[test]
    fn test_score_maintainer_count() {
        let signals = RawSignals {
            maintainer_count: Some(5),
            ..Default::default()
        };
        let score = score_maintainer_count(&signals);
        assert_eq!(score.score, 100.0);
    }

    #[test]
    fn test_score_maintainer_solo() {
        let signals = RawSignals {
            maintainer_count: Some(1),
            ..Default::default()
        };
        let score = score_maintainer_count(&signals);
        assert_eq!(score.score, 20.0);
    }

    #[test]
    fn test_score_download_trend_rising() {
        let signals = RawSignals {
            download_trend: Some(0.5),
            ..Default::default()
        };
        let score = score_download_trend(&signals);
        assert_eq!(score.score, 75.0);
    }

    #[test]
    fn test_score_download_trend_declining() {
        let signals = RawSignals {
            download_trend: Some(-0.5),
            ..Default::default()
        };
        let score = score_download_trend(&signals);
        assert_eq!(score.score, 25.0);
    }

    #[test]
    fn test_score_cve_none() {
        let signals = RawSignals {
            open_cve_count: Some(0),
            ..Default::default()
        };
        let score = score_cve_history(&signals);
        assert_eq!(score.score, 100.0);
    }

    #[test]
    fn test_score_cve_many() {
        let signals = RawSignals {
            open_cve_count: Some(4),
            ..Default::default()
        };
        let score = score_cve_history(&signals);
        assert_eq!(score.score, 0.0);
    }
}
