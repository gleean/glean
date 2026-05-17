//! MCP stdio front-door: newline-delimited JSON-RPC; stdout carries protocol frames only.

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::mcp_protocol::router::{handle_json_line, HandleOutcome, McpSharedState};

/// Run the MCP stdio server (invoked by `glean mcp`).
pub async fn run_mcp_server() -> Result<()> {
    let workspace_root = std::env::var("GLEAN_WORKSPACE_ROOT")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("resolve cwd"));
    let workspace_root = workspace_root.canonicalize().unwrap_or(workspace_root);

    let runtime_config = glean_core::GleanConfig::load_merged().context("load glean config")?;

    crate::logging::init_logging(
        crate::logging::LogRuntime::Mcp,
        Some(runtime_config.log.level.as_str()),
    )
    .context("init logging")?;

    let global = glean_core::open_global().context("open GLEAN_STORAGE_ROOT")?;
    let engine = glean_core::GleanEngine::open_for_workspace(
        &workspace_root,
        global,
        crate::parser_bootstrap::build_parser_registry(),
        runtime_config,
    )
    .await
    .context("open glean engine")?;

    let ctx = McpSharedState {
        engine,
        workspace_root,
    };

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await.context("read stdin")?;
        if bytes == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            continue;
        }
        let outcome: HandleOutcome = handle_json_line(trimmed, &ctx).await;
        match outcome {
            HandleOutcome::Silent => {}
            HandleOutcome::Reply(response) => {
                let out = format!("{response}\n");
                stdout.write_all(out.as_bytes()).await?;
                stdout.flush().await?;
            }
        }
    }

    Ok(())
}
