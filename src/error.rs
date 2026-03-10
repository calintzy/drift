use thiserror::Error;

#[derive(Error, Debug)]
pub enum DriftError {
    #[error("package.json을 찾을 수 없습니다: {path}")]
    PackageJsonNotFound { path: String },

    #[error("package.json 파싱 실패: {reason}")]
    ParseError { reason: String },

    #[error("GitHub API 오류: {status} - {message}")]
    GitHubApiError { status: u16, message: String },

    #[error("GitHub API rate limit 초과 (리셋: {reset_at})")]
    RateLimitExceeded { reset_at: String },

    #[error("npm Registry API 오류: {message}")]
    NpmApiError { message: String },

    #[error("OSV API 오류: {message}")]
    OsvApiError { message: String },

    #[error("네트워크 오류: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("리포지토리 URL을 확인할 수 없습니다: {package}")]
    RepoNotFound { package: String },
}
