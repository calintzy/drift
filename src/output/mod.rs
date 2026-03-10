pub mod json;
pub mod table;

use crate::types::DriftReport;

pub trait OutputFormatter {
    fn format(&self, report: &DriftReport, verbose: bool) -> String;
}
