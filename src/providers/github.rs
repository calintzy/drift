use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde_json::Value;
use tracing::warn;

use crate::types::RawSignals;

use super::SignalProvider;

pub struct GitHubProvider {
    client: Client,
    token: Option<String>,
}

impl GitHubProvider {
    pub fn new(client: Client) -> Self {
        let token = std::env::var("GITHUB_TOKEN").ok();
        Self { client, token }
    }

    fn api_url(&self, path: &str) -> String {
        format!("https://api.github.com{path}")
    }

    fn build_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut req = self
            .client
            .get(url)
            .header("User-Agent", "drift-cli")
            .header("Accept", "application/vnd.github.v3+json");

        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        req
    }

    async fn fetch_json(&self, url: &str) -> Result<Value> {
        let resp = self
            .build_request(url)
            .send()
            .await
            .with_context(|| format!("GitHub API 호출 실패: {url}"))?;

        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            let remaining = resp
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("?");
            if remaining == "0" {
                let reset = resp
                    .headers()
                    .get("x-ratelimit-reset")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown");
                anyhow::bail!("GitHub API rate limit 초과 (리셋: {reset})");
            }
        }

        if !resp.status().is_success() {
            anyhow::bail!("GitHub API 오류: {}", resp.status());
        }

        resp.json().await.context("GitHub API JSON 파싱 실패")
    }

    async fn fetch_last_commit(&self, owner: &str, repo: &str) -> Option<DateTime<Utc>> {
        let url = self.api_url(&format!("/repos/{owner}/{repo}/commits?per_page=1"));
        let json = self.fetch_json(&url).await.ok()?;
        json.as_array()?
            .first()?
            .get("commit")?
            .get("committer")?
            .get("date")?
            .as_str()?
            .parse::<DateTime<Utc>>()
            .ok()
    }

    async fn fetch_repo_info(&self, owner: &str, repo: &str) -> Option<(bool, u64)> {
        let url = self.api_url(&format!("/repos/{owner}/{repo}"));
        let json = self.fetch_json(&url).await.ok()?;
        let archived = json.get("archived")?.as_bool().unwrap_or(false);
        let stars = json.get("stargazers_count")?.as_u64().unwrap_or(0);
        Some((archived, stars))
    }

    async fn fetch_contributor_count(&self, owner: &str, repo: &str) -> Option<u32> {
        let url =
            self.api_url(&format!("/repos/{owner}/{repo}/contributors?per_page=1&anon=false"));
        let resp = self
            .build_request(&url)
            .send()
            .await
            .ok()?;

        // Link 헤더에서 마지막 페이지 번호로 전체 수 추정
        if let Some(link) = resp.headers().get("link").and_then(|v| v.to_str().ok()) {
            if let Some(last) = extract_last_page(link) {
                return Some(last as u32);
            }
        }

        // Link 헤더 없으면 실제 배열 크기 사용
        let json: Value = resp.json().await.ok()?;
        json.as_array().map(|arr| arr.len() as u32)
    }

    async fn fetch_release_frequency(&self, owner: &str, repo: &str) -> Option<f64> {
        let url = self.api_url(&format!("/repos/{owner}/{repo}/releases?per_page=10"));
        let json = self.fetch_json(&url).await.ok()?;
        let releases = json.as_array()?;

        if releases.len() < 2 {
            return Some(0.0);
        }

        let dates: Vec<DateTime<Utc>> = releases
            .iter()
            .filter_map(|r| {
                r.get("published_at")?
                    .as_str()?
                    .parse::<DateTime<Utc>>()
                    .ok()
            })
            .collect();

        if dates.len() < 2 {
            return Some(0.0);
        }

        let first = dates.last()?;
        let last = dates.first()?;
        let months = (*last - *first).num_days() as f64 / 30.0;

        if months <= 0.0 {
            return Some(0.0);
        }

        Some(dates.len() as f64 / months)
    }

    async fn fetch_issue_response_time(&self, owner: &str, repo: &str) -> Option<f64> {
        let url = self.api_url(&format!(
            "/repos/{owner}/{repo}/issues?state=closed&per_page=20&sort=updated"
        ));
        let json = self.fetch_json(&url).await.ok()?;
        let issues = json.as_array()?;

        let mut response_hours: Vec<f64> = Vec::new();

        for issue in issues {
            // PR 제외
            if issue.get("pull_request").is_some() {
                continue;
            }

            let created_at = issue
                .get("created_at")?
                .as_str()?
                .parse::<DateTime<Utc>>()
                .ok()?;

            // 첫 응답 = closed_at (간소화, 실제로는 comments timeline 필요)
            if let Some(closed_str) = issue.get("closed_at").and_then(|v| v.as_str()) {
                if let Ok(closed_at) = closed_str.parse::<DateTime<Utc>>() {
                    let hours = (closed_at - created_at).num_hours() as f64;
                    if hours >= 0.0 {
                        response_hours.push(hours);
                    }
                }
            }
        }

        if response_hours.is_empty() {
            return None;
        }

        response_hours.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = response_hours[response_hours.len() / 2];
        Some(median)
    }

    async fn fetch_pr_merge_rate(&self, owner: &str, repo: &str) -> Option<f64> {
        let url = self.api_url(&format!(
            "/repos/{owner}/{repo}/pulls?state=all&per_page=30&sort=updated"
        ));
        let json = self.fetch_json(&url).await.ok()?;
        let prs = json.as_array()?;

        if prs.is_empty() {
            return None;
        }

        let merged = prs
            .iter()
            .filter(|pr| pr.get("merged_at").and_then(|v| v.as_str()).is_some())
            .count();

        Some(merged as f64 / prs.len() as f64)
    }
}

#[async_trait]
impl SignalProvider for GitHubProvider {
    async fn collect(&self, _package_name: &str, repo_url: Option<&str>) -> Result<RawSignals> {
        let repo_url = match repo_url {
            Some(url) => url,
            None => return Ok(RawSignals::default()),
        };

        let (owner, repo) = match super::npm::extract_github_owner_repo(repo_url) {
            Some(pair) => pair,
            None => {
                warn!("GitHub URL 파싱 실패: {repo_url}");
                return Ok(RawSignals::default());
            }
        };

        // 모든 신호를 병렬로 수집
        let (last_commit, repo_info, contributors, release_freq, issue_response, pr_merge) =
            tokio::join!(
                self.fetch_last_commit(&owner, &repo),
                self.fetch_repo_info(&owner, &repo),
                self.fetch_contributor_count(&owner, &repo),
                self.fetch_release_frequency(&owner, &repo),
                self.fetch_issue_response_time(&owner, &repo),
                self.fetch_pr_merge_rate(&owner, &repo),
            );

        let (is_archived, _stars) = repo_info.unwrap_or((false, 0));

        Ok(RawSignals {
            last_commit,
            release_frequency: release_freq,
            maintainer_count: contributors,
            issue_response_median_hours: issue_response,
            pr_merge_rate: pr_merge,
            is_archived,
            ..Default::default()
        })
    }
}

/// Link 헤더에서 마지막 페이지 번호 추출
fn extract_last_page(link_header: &str) -> Option<usize> {
    for part in link_header.split(',') {
        if part.contains("rel=\"last\"") {
            if let Some(start) = part.find("page=") {
                let rest = &part[start + 5..];
                if let Some(end) = rest.find('>') {
                    return rest[..end].parse().ok();
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_last_page() {
        let header = r#"<https://api.github.com/repos/foo/bar/contributors?page=2>; rel="next", <https://api.github.com/repos/foo/bar/contributors?page=45>; rel="last""#;
        assert_eq!(extract_last_page(header), Some(45));
    }

    #[test]
    fn test_extract_last_page_none() {
        assert_eq!(extract_last_page(""), None);
    }
}
