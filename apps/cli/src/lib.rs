//! Glean CLI as a library, reused by integration tests and the `main` binary.

mod cmd_config;
mod cmd_daemon;
mod cmd_logs;
mod cmd_mcp;
mod cmd_models;
mod cmd_status;
mod logging;
pub mod mcp_protocol;
mod parser_bootstrap;

use std::path::PathBuf;

use anyhow::{Context, Result};
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
    /// Inspect or scaffold TOML configuration (`list`/`set`: merge key; `init`: optional workspace target).
    Config {
        /// Workspace root for `list`/`set` merge (defaults to cwd or `GLEAN_WORKSPACE_ROOT`). For `init`, pass **`--workspace`** only when writing **`<workspace>/.glean/config.toml`**; omit it to write **`$GLEAN_STORAGE_ROOT/config.toml`** (default `~/.glean/config.toml`).
        #[arg(long)]
        workspace: Option<PathBuf>,
        #[command(subcommand)]
        sub: ConfigCommands,
    },
    /// Print rolling log file tail under `GLEAN_STORAGE_ROOT/logs`.
    Logs {
        /// Maximum lines to print from the newest matching log file.
        #[arg(short = 'n', long, default_value_t = 80)]
        lines: usize,
        /// Keep reading and printing new lines (like `tail -f`; Ctrl+C to stop).
        #[arg(short = 'f', long)]
        follow: bool,
        /// Which rolling prefix to prefer (`cli` vs `daemon`).
        #[arg(long, value_enum, default_value_t = cmd_logs::LogRuntimeFilter::Cli)]
        source: cmd_logs::LogRuntimeFilter,
    },
    /// Print version to stderr.
    Status,
    /// Download optional model artifacts into `$GLEAN_STORAGE_ROOT`.
    Models {
        #[command(subcommand)]
        sub: ModelsCommands,
    },
}

#[derive(Subcommand)]
pub enum ModelsCommands {
    /// Fetch model files (e.g. `rerank` → BGE cache under `cache/reranker/`).
    Pull {
        /// Model kind: `rerank`.
        model: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Print merged effective configuration as TOML (stdout).
    #[command(visible_alias = "show")]
    List {
        /// Annotate each TOML section with `default` / `global` / `workspace` provenance.
        #[arg(long, default_value_t = true)]
        show_sources: bool,
        /// Omit section provenance comments (machine-friendly TOML only).
        #[arg(long, conflicts_with = "show_sources")]
        plain: bool,
    },
    /// Write config template: default `$GLEAN_STORAGE_ROOT/config.toml`, or `<workspace>/.glean/config.toml` when `--workspace` is set (`--force` overwrites).
    Init {
        /// Replace the target `config.toml` if it already exists.
        #[arg(long)]
        force: bool,
    },
    /// Set one scalar in workspace or global `config.toml` (creates the file if missing).
    Set {
        /// `SECTION.field`, e.g. `rerank.enabled`, `embedding.device`.
        key: String,
        /// Scalar: `true`, `42`, or a string (quote in shell if it contains spaces).
        value: String,
        /// Write `$GLEAN_STORAGE_ROOT/config.toml` instead of `<workspace>/.glean/config.toml`.
        #[arg(long)]
        global: bool,
    },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Config { workspace, sub } => {
            let ws = cmd_config::resolve_workspace(workspace.clone())?;
            let cfg = glean_core::GleanConfig::load_merged(&ws).ok();
            logging::init_logging(
                logging::LogRuntime::HumanStatus,
                cfg.as_ref().map(|c| c.log.level.as_str()),
            )?;
            match sub {
                ConfigCommands::List {
                    show_sources,
                    plain,
                } => cmd_config::run_config_list(workspace, show_sources && !plain),
                ConfigCommands::Init { force } => cmd_config::run_config_init(workspace, force),
                ConfigCommands::Set { key, value, global } => {
                    cmd_config::run_config_set(workspace, key, value, global)
                }
            }
        }
        Commands::Logs {
            lines,
            source,
            follow,
        } => cmd_logs::run_logs(source, lines, follow),
        Commands::Daemon { workspace } => {
            let ws = cmd_config::resolve_workspace(workspace.clone())?;
            let cfg = glean_core::GleanConfig::load_merged(&ws).context("load glean config")?;
            logging::init_logging(logging::LogRuntime::Daemon, Some(&cfg.log.level))?;
            cmd_daemon::run_daemon(workspace, cfg).await
        }
        Commands::Mcp => cmd_mcp::run_mcp_server().await,
        Commands::Status => cmd_status::run_status().await,
        Commands::Models { sub } => match sub {
            ModelsCommands::Pull { model } => cmd_models::run_models_pull(&model),
        },
    }
}
