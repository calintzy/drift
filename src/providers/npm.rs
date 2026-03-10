use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::types::{PackageMetadata, RawSignals};

use super::{MetadataProvider, SignalProvider};

pub struct NpmProvider {
    client: Client,
}

impl NpmProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// npm 다운로드 추세 계산: 최근 2주 vs 이전 2주
    async fn fetch_download_trend(&self, package_name: &str) -> Result<f64> {
        let url = format!(
            "https://api.npmjs.org/downloads/range/last-month/{}",
            package_name
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("npm 다운로드 API 호출 실패: {package_name}"))?;

        if !resp.status().is_success() {
            anyhow::bail!("npm 다운로드 API 오류: {}", resp.status());
        }

        let json: Value = resp.json().await?;
        let downloads = json
            .get("downloads")
            .and_then(|v| v.as_array())
            .unwrap_or(&Vec::new())
            .clone();

        if downloads.len() < 14 {
            return Ok(0.0);
        }

        let mid = downloads.len() / 2;
        let first_half: f64 = downloads[..mid]
            .iter()
            .filter_map(|d| d.get("downloads").and_then(|v| v.as_f64()))
            .sum();
        let second_half: f64 = downloads[mid..]
            .iter()
            .filter_map(|d| d.get("downloads").and_then(|v| v.as_f64()))
            .sum();

        if first_half == 0.0 {
            return Ok(0.0);
        }

        let trend = (second_half - first_half) / first_half;
        Ok(trend.clamp(-1.0, 1.0))
    }
}

#[async_trait]
impl MetadataProvider for NpmProvider {
    async fn get_metadata(&self, package_name: &str) -> Result<PackageMetadata> {
        let url = format!("https://registry.npmjs.org/{}", package_name);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("npm registry API 호출 실패: {package_name}"))?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "npm registry API 오류: {} for {}",
                resp.status(),
                package_name
            );
        }

        let json: Value = resp.json().await?;

        let repository_url = json
            .get("repository")
            .and_then(|r| r.get("url"))
            .and_then(|u| u.as_str())
            .map(normalize_repo_url);

        let deprecated = json
            .get("versions")
            .and_then(|v| v.as_object())
            .and_then(|versions| {
                versions
                    .values()
                    .next_back()
                    .and_then(|v| v.get("deprecated"))
                    .map(|d| !d.is_null())
            })
            .unwrap_or(false);

        Ok(PackageMetadata {
            name: package_name.to_string(),
            repository_url,
            deprecated,
        })
    }
}

#[async_trait]
impl SignalProvider for NpmProvider {
    async fn collect(&self, package_name: &str, _repo_url: Option<&str>) -> Result<RawSignals> {
        let trend = self.fetch_download_trend(package_name).await.unwrap_or(0.0);

        Ok(RawSignals {
            download_trend: Some(trend),
            ..Default::default()
        })
    }
}

/// git+https://github.com/user/repo.git → user/repo
fn normalize_repo_url(url: &str) -> String {
    // github:user/repo 단축 형식 처리
    if let Some(rest) = url.strip_prefix("github:") {
        return format!("https://github.com/{}", rest);
    }

    url.replace("git+", "")
        .replace("git://", "https://")
        .replace("ssh://git@github.com/", "https://github.com/")
        .replace(".git", "")
        .trim_end_matches('/')
        .to_string()
}

/// GitHub URL에서 owner/repo 추출
pub fn extract_github_owner_repo(url: &str) -> Option<(String, String)> {
    let normalized = normalize_repo_url(url);
    let parts: Vec<&str> = normalized
        .trim_start_matches("https://github.com/")
        .trim_start_matches("http://github.com/")
        .splitn(3, '/')
        .collect();

    if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_repo_url() {
        assert_eq!(
            normalize_repo_url("git+https://github.com/user/repo.git"),
            "https://github.com/user/repo"
        );
        assert_eq!(
            normalize_repo_url("git://github.com/user/repo.git"),
            "https://github.com/user/repo"
        );
    }

    #[test]
    fn test_normalize_github_shorthand() {
        assert_eq!(
            normalize_repo_url("github:user/repo"),
            "https://github.com/user/repo"
        );
    }

    #[test]
    fn test_extract_github_owner_repo() {
        let result = extract_github_owner_repo("https://github.com/facebook/react");
        assert_eq!(result, Some(("facebook".into(), "react".into())));

        let result = extract_github_owner_repo("git+https://github.com/axios/axios.git");
        assert_eq!(result, Some(("axios".into(), "axios".into())));

        let result = extract_github_owner_repo("https://gitlab.com/user/repo");
        assert_eq!(result, None);
    }
}
