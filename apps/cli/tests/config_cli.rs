//! Subprocess tests for `glean config list` / `init` / `set`.

use std::process::Command;

#[test]
fn glean_config_init_writes_global_config_under_storage_root() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");

    let out = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "init"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
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
fn glean_config_set_writes_storage_root_config() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");

    let set = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "set", "log.level", "debug"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .output()
        .expect("spawn set");

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
    let out = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "list"])
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
        stdout.contains("source: default") || stdout.contains("source: global"),
        "expected provenance comments: {stdout:?}"
    );
}

#[test]
fn glean_config_list_plain_omits_provenance() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    let out = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "list", "--plain"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .output()
        .expect("spawn");

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).expect("utf8");
    assert!(!stdout.contains("source: default"));
}

#[test]
fn glean_config_set_updates_merged_list() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");

    let set = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "set", "rerank.enabled", "true"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .output()
        .expect("spawn set");

    assert!(
        set.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&set.stderr)
    );

    let list = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "show"])
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
        "merged output should reflect global set: {merged}"
    );
}

#[test]
fn glean_config_set_rejects_malformed_key() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    let out = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "set", "nosectionfield", "1"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .output()
        .expect("spawn");

    assert!(
        !out.status.success(),
        "expected error, stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn glean_config_set_rejects_unknown_key() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");
    let out = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "set", "embedding.unknown_field", "x"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .output()
        .expect("spawn");

    assert!(
        !out.status.success(),
        "expected error, stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn glean_config_init_refuses_overwrite_without_force() {
    let storage_tmp = tempfile::tempdir().expect("tempdir");

    let first = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "init"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .output()
        .expect("spawn");
    assert!(first.status.success(), "first init should succeed");

    let second = Command::new(env!("CARGO_BIN_EXE_glean"))
        .args(["config", "init"])
        .env("GLEAN_STORAGE_ROOT", storage_tmp.path())
        .output()
        .expect("spawn");
    assert!(
        !second.status.success(),
        "second init without --force should fail, stderr={}",
        String::from_utf8_lossy(&second.stderr)
    );
}
