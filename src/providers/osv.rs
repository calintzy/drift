use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use crate::types::RawSignals;

use super::SignalProvider;

pub struct OsvProvider {
    client: Client,
}

impl OsvProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SignalProvider for OsvProvider {
    async fn collect(&self, package_name: &str, _repo_url: Option<&str>) -> Result<RawSignals> {
        let url = "https://api.osv.dev/v1/query";
        let body = json!({
            "package": {
                "name": package_name,
                "ecosystem": "npm"
            }
        });

        let resp = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("OSV API 호출 실패: {package_name}"))?;

        if !resp.status().is_success() {
            // OSV API 실패 시 CVE 없음으로 처리 (graceful degradation)
            return Ok(RawSignals {
                open_cve_count: Some(0),
                ..Default::default()
            });
        }

        let json: Value = resp.json().await.unwrap_or(json!({}));
        let vulns = json
            .get("vulns")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len() as u32)
            .unwrap_or(0);

        Ok(RawSignals {
            open_cve_count: Some(vulns),
            ..Default::default()
        })
    }
}
