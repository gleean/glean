//! Subprocess smoke tests for `glean mcp` stdio framing.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[test]
fn glean_mcp_initialize_returns_json_line() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    let mut child = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["mcp"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn glean mcp");

    let mut stdin = child.stdin.take().expect("stdin");
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#;
    stdin.write_all(req.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    drop(stdin);

    let output = child.wait_with_output().expect("wait output");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let first = stdout.lines().next().expect("one stdout line");
    let v: serde_json::Value = serde_json::from_str(first).expect("json");
    assert_eq!(v["jsonrpc"], "2.0");
    assert_eq!(v["result"]["serverInfo"]["name"], "glean");
}

#[test]
fn glean_mcp_invalid_json_returns_error_and_exits_cleanly() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    let mut child = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["mcp"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn glean mcp");

    let mut stdin = child.stdin.take().expect("stdin");
    stdin.write_all(b"not-json\n").unwrap();
    drop(stdin);

    let output = child.wait_with_output().expect("wait output");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let first = stdout.lines().next().expect("error line");
    let v: serde_json::Value = serde_json::from_str(first).expect("json");
    assert!(
        v.get("error").is_some(),
        "expected JSON-RPC error object, got {:?}",
        v
    );
}

#[test]
fn glean_mcp_exits_nonzero_when_workspace_config_toml_invalid() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    let ws_tmp = tempfile::tempdir().expect("workspace tempdir");
    let glean_dir: PathBuf = ws_tmp.path().join(".glean");
    std::fs::create_dir_all(&glean_dir).expect("mkdir .glean");
    std::fs::write(glean_dir.join("config.toml"), "not-valid-toml [[[\n").expect("write bad toml");

    let output = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["mcp"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .env("GLEAN_WORKSPACE_ROOT", ws_tmp.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run glean mcp");

    assert!(
        !output.status.success(),
        "expected failure, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("load glean config") || err.contains("config"),
        "stderr should mention config load, got {err}"
    );
}

#[test]
fn glean_mcp_initialize_succeeds_with_workspace_local_config() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    let ws_tmp = tempfile::tempdir().expect("workspace tempdir");
    let glean_dir: PathBuf = ws_tmp.path().join(".glean");
    std::fs::create_dir_all(&glean_dir).expect("mkdir .glean");
    std::fs::write(
        glean_dir.join("config.toml"),
        r#"
[log]
level = "debug"
"#,
    )
    .expect("write config");

    let mut child = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["mcp"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .env("GLEAN_WORKSPACE_ROOT", ws_tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn glean mcp");

    let mut stdin = child.stdin.take().expect("stdin");
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#;
    stdin.write_all(req.as_bytes()).unwrap();
    stdin.write_all(b"\n").unwrap();
    drop(stdin);

    let output = child.wait_with_output().expect("wait output");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let first = stdout.lines().next().expect("one stdout line");
    let v: serde_json::Value = serde_json::from_str(first).expect("json");
    assert_eq!(v["result"]["serverInfo"]["name"], "glean");
}
