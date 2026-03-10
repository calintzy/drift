use crate::types::DriftReport;

use super::OutputFormatter;

pub struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
    fn format(&self, report: &DriftReport, _verbose: bool) -> String {
        serde_json::to_string_pretty(report)
            .unwrap_or_else(|e| format!("{{\"error\": \"JSON 직렬화 실패: {e}\"}}"))
    }
}
