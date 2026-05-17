//! JSON-RPC routing for MCP-facing methods.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{json, Value};

use crate::mcp::types::{jsonrpc_error, parse_error_response, JsonRpcRequest};
use glean_core::GleanEngine;

/// Shared MCP runtime (storage engine + workspace boundary).
pub struct McpSharedState {
    pub engine: Arc<GleanEngine>,
    pub workspace_root: PathBuf,
}

/// Outcome for one stdin line: optionally emit one stdout JSON line.
pub enum HandleOutcome {
    /// No stdout emission (e.g. MCP notifications).
    Silent,
    /// Emit exactly one JSON line on stdout.
    Reply(String),
}

pub async fn handle_json_line(line: &str, ctx: &McpSharedState) -> HandleOutcome {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return HandleOutcome::Silent;
    }

    let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
        Ok(r) => r,
        Err(e) => {
            return HandleOutcome::Reply(parse_error_response(e));
        }
    };

    if req.jsonrpc != "2.0" {
        return HandleOutcome::Reply(jsonrpc_error(
            req.id.unwrap_or(Value::Null),
            -32600,
            "Invalid Request",
            Some(json!({"detail": "jsonrpc must be \"2.0\""})),
        ));
    }

    if req.id.is_none() {
        match req.method.as_str() {
            "notifications/initialized" | "notifications/cancelled" => {
                return HandleOutcome::Silent;
            }
            _ => {
                return HandleOutcome::Reply(jsonrpc_error(
                    Value::Null,
                    -32600,
                    "Invalid Request",
                    Some(json!({"detail": "missing id for non-notification"})),
                ));
            }
        }
    }

    let id = req.id.clone().unwrap_or(Value::Null);

    match req.method.as_str() {
        "initialize" => HandleOutcome::Reply(ok_response(id, initialize_result())),
        "tools/list" => HandleOutcome::Reply(ok_response(id, tools_list_result())),
        "tools/call" => HandleOutcome::Reply(handle_tools_call(id, req.params, ctx).await),
        _ => HandleOutcome::Reply(jsonrpc_error(
            id,
            -32601,
            "Method not found",
            Some(json!({"method": req.method})),
        )),
    }
}

fn ok_response(id: Value, result: Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
    .to_string()
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "glean",
            "version": glean_core::VERSION
        }
    })
}

fn tools_list_result() -> Value {
    json!({
        "tools": [
            tool_descriptor(
                "search_semantic",
                "Semantic search over indexed local documents (Lance hybrid: BM25 full-text on `text` plus vector kNN with RRF fusion when an FTS index exists; falls back to vector-only). Result paths are absolute file paths as stored in the index (canonical workspace root + relative path key).",
                json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query" }
                    },
                    "required": ["query"]
                }),
            ),
            tool_descriptor(
                "read_file_context",
                "Load plain-text context for a file path under GLEAN_WORKSPACE_ROOT.",
                json!({
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Absolute path on disk" }
                    },
                    "required": ["file_path"]
                }),
            ),
            tool_descriptor(
                "get_recent_changes",
                "List indexed paths (workspace-relative POSIX keys) ordered by last observed mtime (SQLite shadow metadata).",
                json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
        ]
    })
}

fn tool_descriptor(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

async fn handle_tools_call(id: Value, params: Option<Value>, ctx: &McpSharedState) -> String {
    let Some(params) = params else {
        return jsonrpc_error(
            id,
            -32602,
            "Invalid params",
            Some(json!({"detail": "missing params"})),
        );
    };

    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    let Some(tool_name) = name else {
        return jsonrpc_error(
            id,
            -32602,
            "Invalid params",
            Some(json!({"detail": "missing string field name"})),
        );
    };

    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match tool_name.as_str() {
        "search_semantic" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            match ctx.engine.semantic_search(query, 32).await {
                Ok(hits) => {
                    let preview: Vec<Value> = hits
                        .into_iter()
                        .map(|(path, text)| {
                            json!({
                                "path": path,
                                "preview": text.chars().take(240).collect::<String>(),
                            })
                        })
                        .collect();
                    let tool_body = json!({
                        "query": query,
                        "results": preview,
                    });
                    ok_response(
                        id,
                        json!({
                            "content": [{
                                "type": "text",
                                "text": tool_body.to_string(),
                            }],
                            "isError": false,
                        }),
                    )
                }
                Err(e) => ok_response(
                    id,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": json!({"error": e.to_string()}).to_string(),
                        }],
                        "isError": true,
                    }),
                ),
            }
        }
        "read_file_context" => {
            let Some(fp) = args.get("file_path").and_then(|v| v.as_str()) else {
                return jsonrpc_error(
                    id,
                    -32602,
                    "Invalid params",
                    Some(json!({"detail": "missing arguments.file_path"})),
                );
            };
            let path = PathBuf::from(fp);
            let (_, max_read) = ctx.engine.runtime_config().indexing.sync_byte_limits();
            match ctx
                .engine
                .read_file_context(&ctx.workspace_root, &path, max_read)
            {
                Ok(text) => ok_response(
                    id,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": json!({"path": fp, "text": text}).to_string(),
                        }],
                        "isError": false,
                    }),
                ),
                Err(e) => ok_response(
                    id,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": json!({"error": e.to_string()}).to_string(),
                        }],
                        "isError": true,
                    }),
                ),
            }
        }
        "get_recent_changes" => match ctx.engine.recent_changes(50) {
            Ok(rows) => {
                let items: Vec<Value> = rows
                    .into_iter()
                    .map(|(path, mtime_ns)| json!({"path": path, "mtime_ns": mtime_ns}))
                    .collect();
                let tool_body = json!({ "items": items });
                ok_response(
                    id,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": tool_body.to_string(),
                        }],
                        "isError": false,
                    }),
                )
            }
            Err(e) => ok_response(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": json!({"error": e.to_string()}).to_string(),
                    }],
                    "isError": true,
                }),
            ),
        },
        other => jsonrpc_error(id, -32602, "Unknown tool", Some(json!({"name": other}))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn lite_ctx() -> McpSharedState {
        use std::sync::Arc;

        let workspace_path = tempfile::tempdir().unwrap().keep();
        let layout = glean_core::WorkspaceIndexLayout::for_workspace(&workspace_path);
        let engine = glean_core::GleanEngine::open_with_embedder(
            layout,
            Arc::new(glean_core::DeterministicEmbedder::new()),
        )
        .await
        .unwrap();
        McpSharedState {
            engine,
            workspace_root: workspace_path,
        }
    }

    async fn sample_ctx() -> McpSharedState {
        let ctx = lite_ctx().await;
        std::fs::write(ctx.workspace_root.join("note.txt"), "hello needle-token").unwrap();
        glean_core::pipeline::run_incremental_sync(ctx.engine.as_ref(), &ctx.workspace_root)
            .await
            .unwrap();
        ctx
    }

    #[tokio::test]
    async fn initialize_returns_server_info() {
        let ctx = sample_ctx().await;
        let line = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
        let HandleOutcome::Reply(s) = handle_json_line(line, &ctx).await else {
            panic!("expected reply");
        };
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 1);
        assert_eq!(v["result"]["serverInfo"]["name"], "glean");
    }

    #[tokio::test]
    async fn notification_initialized_is_silent() {
        let ctx = sample_ctx().await;
        let line = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        match handle_json_line(line, &ctx).await {
            HandleOutcome::Silent => {}
            _ => panic!("expected silent"),
        }
    }

    #[tokio::test]
    async fn tools_list_three_tools_only() {
        let ctx = sample_ctx().await;
        let line = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
        let HandleOutcome::Reply(s) = handle_json_line(line, &ctx).await else {
            panic!("expected reply");
        };
        let v: Value = serde_json::from_str(&s).unwrap();
        let tools = v["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 3);
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(
            names,
            vec!["search_semantic", "read_file_context", "get_recent_changes"]
        );
    }

    #[tokio::test]
    async fn plaintext_initialize_is_parse_error() {
        let ctx = lite_ctx().await;
        let HandleOutcome::Reply(s) = handle_json_line("initialize", &ctx).await else {
            panic!("expected parse error reply");
        };
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["error"]["code"], -32700);
    }

    #[tokio::test]
    async fn unknown_method_returns_jsonrpc_method_not_found() {
        let ctx = lite_ctx().await;
        let line = r#"{"jsonrpc":"2.0","id":99,"method":"does/not_exist"}"#;
        let HandleOutcome::Reply(s) = handle_json_line(line, &ctx).await else {
            panic!("expected error reply");
        };
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn initialize_without_jsonrpc_id_returns_invalid_request() {
        let ctx = lite_ctx().await;
        let line = r#"{"jsonrpc":"2.0","method":"initialize","params":{}}"#;
        let HandleOutcome::Reply(s) = handle_json_line(line, &ctx).await else {
            panic!("expected error reply");
        };
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["error"]["code"], -32600);
    }

    #[tokio::test]
    async fn tools_call_search_finds_indexed_token() {
        let ctx = sample_ctx().await;
        let line = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"search_semantic","arguments":{"query":"needle-token"}}}"#;
        let HandleOutcome::Reply(s) = handle_json_line(line, &ctx).await else {
            panic!("expected reply");
        };
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["result"]["isError"], false);
        let text = v["result"]["content"][0]["text"].as_str().unwrap();
        assert!(
            text.contains("needle-token"),
            "expected hit payload, got {text}"
        );
        assert!(!text.contains("stub: indexing engine not wired yet"));
    }
}
