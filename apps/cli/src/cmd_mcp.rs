//! MCP stdio front-door: newline-delimited JSON-RPC; stdout carries protocol frames only.

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::mcp_protocol::router::{handle_json_line, McpSharedState};

/// Run the MCP stdio server (invoked by `glean mcp`).
pub async fn run_mcp_server() -> Result<()> {
    let workspace_root = std::env::var("GLEAN_WORKSPACE_ROOT")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("resolve cwd"));
    let workspace_root = workspace_root.canonicalize().unwrap_or(workspace_root);

    let runtime_config =
        glean_core::GleanConfig::load_merged(&workspace_root).context("load glean config")?;

    crate::logging::init_logging(
        crate::logging::LogRuntime::Mcp,
        Some(runtime_config.log.level.as_str()),
    )
    .context("init logging")?;

    let layout = glean_core::open_storage().context("open GLEAN_STORAGE_ROOT")?;
    let engine = glean_core::GleanEngine::open_with_registry_and_config(
        layout,
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

        match handle_json_line(line.as_str(), &ctx).await {
            crate::mcp_protocol::router::HandleOutcome::Silent => {}
            crate::mcp_protocol::router::HandleOutcome::Reply(json) => {
                stdout
                    .write_all(json.as_bytes())
                    .await
                    .context("write stdout")?;
                stdout.write_all(b"\n").await.context("write newline")?;
                stdout.flush().await.context("flush stdout")?;
            }
        }
    }

    Ok(())
}
