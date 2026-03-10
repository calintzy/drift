pub mod github;
pub mod npm;
pub mod osv;

use anyhow::Result;
use async_trait::async_trait;

use crate::types::{PackageMetadata, RawSignals};

#[async_trait]
pub trait SignalProvider: Send + Sync {
    async fn collect(&self, package_name: &str, repo_url: Option<&str>) -> Result<RawSignals>;
}

#[async_trait]
pub trait MetadataProvider: Send + Sync {
    async fn get_metadata(&self, package_name: &str) -> Result<PackageMetadata>;
}
