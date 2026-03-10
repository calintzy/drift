use chrono::{DateTime, Utc};
use serde::Serialize;

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
    pub deprecated: bool,
}

/// 7개 신호의 원시 데이터
#[derive(Debug, Clone, Default)]
pub struct RawSignals {
    pub last_commit: Option<DateTime<Utc>>,
    pub release_frequency: Option<f64>,
    pub maintainer_count: Option<u32>,
    pub issue_response_median_hours: Option<f64>,
    pub download_trend: Option<f64>,
    pub open_cve_count: Option<u32>,
    pub star_trend: Option<f64>,
    pub pr_merge_rate: Option<f64>,
    pub is_archived: bool,
    pub is_deprecated: bool,
}

/// 개별 신호 점수 (0~100)
#[derive(Debug, Clone, Serialize)]
pub struct SignalScore {
    pub name: &'static str,
    pub weight: f64,
    pub score: f64,
    pub available: bool,
    pub detail: String,
}

/// 패키지별 최종 건강 보고서
#[derive(Debug, Clone, Serialize)]
pub struct PackageReport {
    pub name: String,
    pub version: String,
    pub health_score: f64,
    pub grade: RiskGrade,
    pub signal_scores: Vec<SignalScore>,
    pub summary_signal: String,
}

/// 리스크 등급
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum RiskGrade {
    Safe,
    Watch,
    Risk,
    Dead,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_grade_safe_boundary() {
        assert_eq!(RiskGrade::from_score(100.0), RiskGrade::Safe);
        assert_eq!(RiskGrade::from_score(80.0), RiskGrade::Safe);
        assert_eq!(RiskGrade::from_score(79.0), RiskGrade::Watch);
    }

    #[test]
    fn test_risk_grade_watch_boundary() {
        assert_eq!(RiskGrade::from_score(60.0), RiskGrade::Watch);
        assert_eq!(RiskGrade::from_score(59.0), RiskGrade::Risk);
    }

    #[test]
    fn test_risk_grade_risk_boundary() {
        assert_eq!(RiskGrade::from_score(40.0), RiskGrade::Risk);
        assert_eq!(RiskGrade::from_score(39.0), RiskGrade::Dead);
    }

    #[test]
    fn test_risk_grade_dead() {
        assert_eq!(RiskGrade::from_score(0.0), RiskGrade::Dead);
        assert_eq!(RiskGrade::from_score(10.0), RiskGrade::Dead);
    }
}
