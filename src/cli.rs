use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "drift",
    version,
    about = "Dependency health monitor — know before your dependencies die"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 의존성 건강 점수 체크
    Check {
        /// 특정 패키지만 체크 (생략 시 전체)
        packages: Vec<String>,

        /// 출력 포맷
        #[arg(long, default_value = "table")]
        format: OutputFormat,

        /// devDependencies 포함
        #[arg(long)]
        include_dev: bool,

        /// 상세 신호 점수 표시
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Table,
    Json,
}
