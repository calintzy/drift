pub mod package_json;

use std::path::Path;

use anyhow::Result;

use crate::types::Dependency;

pub trait DependencyParser {
    fn parse(&self, path: &Path, include_dev: bool) -> Result<Vec<Dependency>>;
    fn detect(&self, path: &Path) -> bool;
}
