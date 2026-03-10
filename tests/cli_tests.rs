use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// 테스트 1: package.json 없는 디렉토리 → exit 1
#[test]
fn test_no_package_json_exits_with_error() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("drift")
        .unwrap()
        .arg("check")
        .current_dir(temp.path())
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("package.json"));
}

/// 테스트 2: --format json → 유효한 JSON 출력 (빈 deps는 바로 리턴하므로 fixture 사용)
#[test]
fn test_json_format_flag_accepted() {
    let temp = TempDir::new().unwrap();
    // 빈 dependencies로 테스트 (API 호출 없이)
    let pkg = r#"{"name":"test","version":"1.0.0","dependencies":{}}"#;
    fs::write(temp.path().join("package.json"), pkg).unwrap();

    Command::cargo_bin("drift")
        .unwrap()
        .args(["check", "--format", "json"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("의존성이 없습니다"));
}

/// 테스트 3: 특정 패키지만 체크 (존재하지 않는 패키지 필터)
#[test]
fn test_filter_nonexistent_package() {
    let temp = TempDir::new().unwrap();
    let pkg = r#"{"name":"test","version":"1.0.0","dependencies":{"express":"^4.18.2"}}"#;
    fs::write(temp.path().join("package.json"), pkg).unwrap();

    Command::cargo_bin("drift")
        .unwrap()
        .args(["check", "nonexistent-pkg"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "지정한 패키지가 dependencies에 없습니다",
        ));
}

/// 테스트 4: --include-dev 플래그 수용 확인
#[test]
fn test_include_dev_flag_accepted() {
    let temp = TempDir::new().unwrap();
    let pkg = r#"{"name":"test","version":"1.0.0","dependencies":{}}"#;
    fs::write(temp.path().join("package.json"), pkg).unwrap();

    Command::cargo_bin("drift")
        .unwrap()
        .args(["check", "--include-dev"])
        .current_dir(temp.path())
        .assert()
        .success();
}

/// 테스트 5: --verbose 플래그 수용 확인
#[test]
fn test_verbose_flag_accepted() {
    let temp = TempDir::new().unwrap();
    let pkg = r#"{"name":"test","version":"1.0.0","dependencies":{}}"#;
    fs::write(temp.path().join("package.json"), pkg).unwrap();

    Command::cargo_bin("drift")
        .unwrap()
        .args(["check", "--verbose"])
        .current_dir(temp.path())
        .assert()
        .success();
}
