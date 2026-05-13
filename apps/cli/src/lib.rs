//! Glean CLI as a library, reused by integration tests and the `main` binary.

mod cmd_daemon;
mod cmd_logs;
mod cmd_mcp;
mod logging;
pub mod mcp_protocol;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "glean",
    version = glean_core::VERSION,
    about = "Local-first knowledge engine CLI"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Long-running indexing daemon (watch workspace + incremental sync).
    Daemon {
        /// Workspace root to scan / watch (defaults to cwd).
        #[arg(long, env = "GLEAN_WORKSPACE_ROOT")]
        workspace: Option<PathBuf>,
    },
    /// MCP server over stdio (JSON-RPC 2.0).
    Mcp,
    /// Print rolling log file tail under `GLEAN_STORAGE_ROOT/logs`.
    Logs {
        /// Maximum lines to print from the newest matching log file.
        #[arg(short = 'n', long, default_value_t = 80)]
        lines: usize,
        /// Which rolling prefix to prefer (`cli` vs `daemon`).
        #[arg(long, value_enum, default_value_t = cmd_logs::LogRuntimeFilter::Cli)]
        source: cmd_logs::LogRuntimeFilter,
    },
    /// Print version to stderr.
    Status,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Logs { lines, source } => cmd_logs::run_logs(source, lines),
        Commands::Daemon { workspace } => {
            logging::init_logging(logging::LogRuntime::Daemon)?;
            cmd_daemon::run_daemon(workspace).await
        }
        Commands::Mcp => {
            logging::init_logging(logging::LogRuntime::Mcp)?;
            cmd_mcp::run_mcp_server().await
        }
        Commands::Status => {
            logging::init_logging(logging::LogRuntime::HumanStatus)?;
            tracing::info!("glean {}", glean_core::VERSION);
            Ok(())
        }
    }
}
