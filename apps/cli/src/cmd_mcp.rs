//! MCP stdio front-door: newline-delimited JSON-RPC; stdout carries protocol frames only.

use anyhow::{Context, Result};
use glean_host::mcp::router::{handle_json_line, HandleOutcome, McpSharedState};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Run the MCP stdio server (invoked by `glean mcp`).
pub async fn run_mcp_server() -> Result<()> {
    let workspace_root =
        glean_host::workspace::resolve_workspace_from_env().map_err(|e| anyhow::anyhow!(e))?;

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
        glean_host::parsers::build_default_registry(),
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
