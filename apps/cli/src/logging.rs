//! Install a global `tracing` subscriber once per process (`glean` binary).

use anyhow::{Context, Result};
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;

/// Distinguishes stderr verbosity and rolling log file prefix (`cli` vs `daemon`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogRuntime {
    /// MCP stdio server — protocol uses stdout; logs go to stderr + `cli.log.*`.
    Mcp,
    /// Background indexer — file-first; stderr defaults to `warn` unless `RUST_LOG` is set.
    Daemon,
    /// Short-lived human command (`glean status`) — compact stderr + same rolling file as CLI.
    HumanStatus,
}

fn default_file_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

fn default_daemon_stderr_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
}

/// Build `{GLEAN_STORAGE_ROOT}/logs`, rolling appenders, stderr layer; leaks worker guard for process lifetime.
pub fn init_logging(mode: LogRuntime) -> Result<()> {
    let layout =
        glean_core::StorageLayout::from_env_or_default().context("resolve GLEAN_STORAGE_ROOT")?;
    let log_dir = layout.root.join("logs");
    std::fs::create_dir_all(&log_dir).context("create logs directory")?;

    let file_prefix = match mode {
        LogRuntime::Daemon => "daemon",
        LogRuntime::Mcp | LogRuntime::HumanStatus => "cli",
    };

    let file_appender = tracing_appender::rolling::daily(&log_dir, file_prefix);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let file_filter = default_file_filter();

    let init_result = match mode {
        LogRuntime::HumanStatus => {
            let stderr_filter = file_filter.clone();
            let file_layer = fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_filter(file_filter);
            let stderr_layer = fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(true)
                .without_time()
                .with_level(false)
                .with_target(false)
                .with_filter(stderr_filter);

            tracing_subscriber::registry()
                .with(file_layer)
                .with(stderr_layer)
                .try_init()
        }
        LogRuntime::Mcp => {
            let stderr_filter = file_filter.clone();
            let file_layer = fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_filter(file_filter);
            let stderr_layer = fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(true)
                .with_filter(stderr_filter);

            tracing_subscriber::registry()
                .with(file_layer)
                .with(stderr_layer)
                .try_init()
        }
        LogRuntime::Daemon => {
            let stderr_filter = default_daemon_stderr_filter();
            let file_layer = fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_filter(file_filter);
            let stderr_layer = fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false)
                .with_filter(stderr_filter);

            tracing_subscriber::registry()
                .with(file_layer)
                .with(stderr_layer)
                .try_init()
        }
    };

    init_result.map_err(|e| anyhow::anyhow!("tracing subscriber init: {e}"))?;

    std::mem::forget(guard);
    Ok(())
}
