use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::types::{DepType, Dependency};

use super::DependencyParser;

pub struct PackageJsonParser;

impl DependencyParser for PackageJsonParser {
    fn detect(&self, path: &Path) -> bool {
        path.join("package.json").exists()
    }

    fn parse(&self, path: &Path, include_dev: bool) -> Result<Vec<Dependency>> {
        let file_path = path.join("package.json");
        let content = std::fs::read_to_string(&file_path)
            .with_context(|| format!("package.json 읽기 실패: {}", file_path.display()))?;

        let json: Value =
            serde_json::from_str(&content).with_context(|| "package.json 파싱 실패")?;

        let mut deps = Vec::new();

        if let Some(dependencies) = json.get("dependencies").and_then(|v| v.as_object()) {
            for (name, version) in dependencies {
                let version_str = version.as_str().unwrap_or("*");
                if version_str.starts_with("workspace:") {
                    continue;
                }
                deps.push(Dependency {
                    name: name.clone(),
                    version: version_str.to_string(),
                    dep_type: DepType::Production,
                });
            }
        }

        if include_dev
            && let Some(dev_deps) = json.get("devDependencies").and_then(|v| v.as_object())
        {
            for (name, version) in dev_deps {
                let version_str = version.as_str().unwrap_or("*");
                if version_str.starts_with("workspace:") {
                    continue;
                }
                deps.push(Dependency {
                    name: name.clone(),
                    version: version_str.to_string(),
                    dep_type: DepType::Development,
                });
            }
        }

        Ok(deps)
    }
}

/// package.json에서 프로젝트 이름 추출
pub fn get_project_name(path: &Path) -> String {
    let file_path = path.join("package.json");
    std::fs::read_to_string(&file_path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|json| json.get("name").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_else(|| "unknown-project".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_dependencies() {
        let dir = TempDir::new().unwrap();
        let pkg = r#"{
            "name": "test-project",
            "dependencies": {
                "react": "^18.0.0",
                "axios": "^1.0.0"
            },
            "devDependencies": {
                "typescript": "^5.0.0"
            }
        }"#;
        fs::write(dir.path().join("package.json"), pkg).unwrap();

        let parser = PackageJsonParser;
        let deps = parser.parse(dir.path(), false).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().all(|d| d.dep_type == DepType::Production));
    }

    #[test]
    fn test_parse_with_dev_deps() {
        let dir = TempDir::new().unwrap();
        let pkg = r#"{
            "name": "test-project",
            "dependencies": { "react": "^18.0.0" },
            "devDependencies": { "typescript": "^5.0.0" }
        }"#;
        fs::write(dir.path().join("package.json"), pkg).unwrap();

        let parser = PackageJsonParser;
        let deps = parser.parse(dir.path(), true).unwrap();
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_skip_workspace_protocol() {
        let dir = TempDir::new().unwrap();
        let pkg = r#"{
            "name": "test-project",
            "dependencies": {
                "shared": "workspace:*",
                "react": "^18.0.0"
            }
        }"#;
        fs::write(dir.path().join("package.json"), pkg).unwrap();

        let parser = PackageJsonParser;
        let deps = parser.parse(dir.path(), false).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "react");
    }

    #[test]
    fn test_detect() {
        let dir = TempDir::new().unwrap();
        let parser = PackageJsonParser;
        assert!(!parser.detect(dir.path()));

        fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert!(parser.detect(dir.path()));
    }

    #[test]
    fn test_empty_dependencies() {
        let dir = TempDir::new().unwrap();
        let pkg = r#"{ "name": "empty-project" }"#;
        fs::write(dir.path().join("package.json"), pkg).unwrap();

        let parser = PackageJsonParser;
        let deps = parser.parse(dir.path(), false).unwrap();
        assert!(deps.is_empty());
    }
}
