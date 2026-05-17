//! Subprocess tests for `glean config list` / `init` / `set`.

use std::process::Command;

#[test]
fn glean_config_init_writes_global_config_under_storage_root() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    let ws_tmp = tempfile::tempdir().expect("tempdir");

    let out = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "init"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .env("GLEAN_WORKSPACE_ROOT", ws_tmp.path())
        .output()
        .expect("spawn init");

    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let path = storage_tmp.path().join("config.toml");
    assert!(path.is_file(), "expected {}", path.display());
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.contains("[rerank]"), "template: {text}");
}

#[test]
fn glean_config_init_with_workspace_writes_project_dot_glean() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    let ws_tmp = tempfile::tempdir().expect("tempdir");
    let ws = ws_tmp.path();

    let out = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "--workspace", ws.to_str().unwrap(), "init"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .output()
        .expect("spawn init");

    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let path = ws.join(".glean").join("config.toml");
    assert!(path.is_file(), "expected {}", path.display());
}

#[test]
fn glean_config_set_global_writes_storage_root_config() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    let ws_tmp = tempfile::tempdir().expect("tempdir");

    let set = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "set", "--global", "log.level", "debug"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .env("GLEAN_WORKSPACE_ROOT", ws_tmp.path())
        .output()
        .expect("spawn set global");

    assert!(
        set.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&set.stderr)
    );

    let path = storage_tmp.path().join("config.toml");
    let text = std::fs::read_to_string(&path).expect("read global config");
    assert!(text.contains("level = \"debug\"") || text.contains("level = 'debug'"));
}

#[test]
fn glean_config_list_prints_toml() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    let ws_tmp = tempfile::tempdir().expect("tempdir");
    let out = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args([
            "config",
            "--workspace",
            ws_tmp.path().to_str().unwrap(),
            "list",
        ])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .output()
        .expect("spawn");

    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf8");
    assert!(
        stdout.contains("rerank") || stdout.contains("embedding"),
        "expected config sections in stdout: {stdout:?}"
    );
    assert!(
        stdout.contains("source: default") || stdout.contains("source: workspace"),
        "expected provenance comments: {stdout:?}"
    );
}

#[test]
fn glean_config_list_plain_omits_provenance() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    let ws_tmp = tempfile::tempdir().expect("tempdir");
    let out = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args([
            "config",
            "--workspace",
            ws_tmp.path().to_str().unwrap(),
            "list",
            "--plain",
        ])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .output()
        .expect("spawn");

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).expect("utf8");
    assert!(!stdout.contains("source: default"));
}

#[test]
fn glean_config_set_creates_workspace_override() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    let ws_tmp = tempfile::tempdir().expect("tempdir");
    let ws = ws_tmp.path();

    let set = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args([
            "config",
            "--workspace",
            ws.to_str().unwrap(),
            "set",
            "rerank.enabled",
            "true",
        ])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .output()
        .expect("spawn set");

    assert!(
        set.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&set.stderr)
    );

    let path = ws.join(".glean").join("config.toml");
    let text = std::fs::read_to_string(&path).expect("read config");
    assert!(
        text.contains("enabled = true") || text.contains("enabled=true"),
        "expected rerank.enabled true in file: {text}"
    );

    let list = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "--workspace", ws.to_str().unwrap(), "show"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .output()
        .expect("spawn list alias");

    assert!(
        list.status.success(),
        "{}",
        String::from_utf8_lossy(&list.stderr)
    );
    let merged = String::from_utf8(list.stdout).expect("utf8");
    assert!(
        merged.contains("enabled = true") || merged.contains("enabled=true"),
        "merged output should reflect workspace override: {merged}"
    );
}
