//! `glean config`: print merged config, scaffold global or workspace TOML, or patch a workspace key.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use toml::Value;

fn resolve_workspace(workspace: Option<PathBuf>) -> Result<PathBuf> {
    let root = workspace
        .or_else(|| {
            std::env::var("GLEAN_WORKSPACE_ROOT")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));
    Ok(root.canonicalize().unwrap_or(root))
}

fn workspace_config_path(workspace: &std::path::Path) -> PathBuf {
    workspace.join(".glean").join("config.toml")
}

/// Default template: mirrors `GleanConfig::default()` (see `glean-core` `config/mod.rs`).
const CONFIG_INIT_TEMPLATE: &str = r#"# Glean TOML (merge order: defaults → $GLEAN_STORAGE_ROOT/config.toml → <workspace>/.glean/config.toml).
# See repository README for environment variables.

[core]
path = "."
threads = 0

[indexing]
watch_interval = 10
max_file_size = 10485760
use_gitignore = true

[embedding]
model = "all-MiniLM-L6-v2"
dimension = 384
device = "cpu"

[rerank]
enabled = false
top_k = 20
model_path = "models/reranker/bge-v2-m3.onnx"

[log]
level = "info"
mask_privacy = true
"#;

fn parse_key_path(key: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = key.trim().split('.').filter(|s| !s.is_empty()).collect();
    if parts.len() != 2 {
        bail!("KEY must be SECTION.FIELD (exactly one dot), e.g. rerank.enabled, embedding.device");
    }
    Ok((parts[0].to_ascii_lowercase(), parts[1].to_ascii_lowercase()))
}

fn validate_key(section: &str, field: &str) -> Result<()> {
    let ok = matches!(
        (section, field),
        ("core", "path" | "threads")
            | (
                "indexing",
                "watch_interval" | "max_file_size" | "use_gitignore"
            )
            | ("embedding", "model" | "dimension" | "device")
            | ("rerank", "enabled" | "top_k" | "model_path")
            | ("log", "level" | "mask_privacy")
    );
    if !ok {
        bail!(
            "unknown key `{section}.{field}`; allowed keys match `GleanConfig` (see `glean config list`)"
        );
    }
    Ok(())
}

fn parse_scalar(raw: &str) -> Result<Value> {
    let t = raw.trim();
    if t.eq_ignore_ascii_case("true") {
        return Ok(Value::Boolean(true));
    }
    if t.eq_ignore_ascii_case("false") {
        return Ok(Value::Boolean(false));
    }
    if let Ok(i) = t.parse::<i64>() {
        return Ok(Value::Integer(i));
    }
    if (t.starts_with('"') && t.ends_with('"') && t.len() >= 2)
        || (t.starts_with('\'') && t.ends_with('\'') && t.len() >= 2)
    {
        return Ok(Value::String(t[1..t.len() - 1].to_string()));
    }
    Ok(Value::String(t.to_string()))
}

/// Print merged `GleanConfig` as TOML (stdout).
pub fn run_config_list(workspace: Option<PathBuf>) -> Result<()> {
    let workspace = resolve_workspace(workspace)?;
    let cfg =
        glean_core::GleanConfig::load_merged(&workspace).context("load merged Glean config")?;
    let text =
        toml::to_string(&cfg).context("serialize effective config as TOML (internal error)")?;
    print!("{text}");
    Ok(())
}

/// Initialize a config file from the built-in template (`--force` overwrites).
///
/// - **No `--workspace`** on `glean config`: writes **`$GLEAN_STORAGE_ROOT/config.toml`** (default **`~/.glean/config.toml`**).
/// - **With `--workspace`**: writes **`<workspace>/.glean/config.toml`**.
///
/// After writing, runs `GleanConfig::load_merged` using the resolved workspace (cwd / `GLEAN_WORKSPACE_ROOT` / `--workspace`) to validate the merge.
pub fn run_config_init(workspace_flag: Option<PathBuf>, force: bool) -> Result<()> {
    let merge_workspace = resolve_workspace(workspace_flag.clone())?;
    let path = if workspace_flag.is_some() {
        let ws = resolve_workspace(workspace_flag)?;
        ws.join(".glean").join("config.toml")
    } else {
        let layout =
            glean_core::StorageLayout::from_env_or_default().map_err(|e| anyhow::anyhow!(e))?;
        layout.root.join("config.toml")
    };

    if path.is_file() && !force {
        bail!(
            "refusing to overwrite {}; pass --force to replace",
            path.display()
        );
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    std::fs::write(&path, CONFIG_INIT_TEMPLATE.as_bytes())
        .with_context(|| format!("write {}", path.display()))?;

    glean_core::GleanConfig::load_merged(&merge_workspace).with_context(|| {
        format!(
            "written file parses, but merged load failed (check the other config layer): {}",
            path.display()
        )
    })?;

    eprintln!("Wrote {}", path.display());
    Ok(())
}

/// Set a single scalar in `<workspace>/.glean/config.toml` (creates file and parent dirs if needed).
///
/// `KEY` is `section.field` (e.g. `rerank.enabled`). Values: `true`/`false`, integers, or strings (quote for spaces).
pub fn run_config_set(workspace: Option<PathBuf>, key: String, value: String) -> Result<()> {
    let workspace = resolve_workspace(workspace)?;
    let (section, field) = parse_key_path(&key)?;
    validate_key(&section, &field)?;

    let path = workspace_config_path(&workspace);
    let mut root = if path.is_file() {
        let s =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let v: Value = toml::from_str(&s)
            .with_context(|| format!("parse workspace config {}", path.display()))?;
        match v {
            Value::Table(_) => v,
            _ => bail!("{}: root must be a TOML table", path.display()),
        }
    } else {
        Value::Table(toml::Table::new())
    };

    let root_map = root.as_table_mut().expect("root is table");
    let section_entry = root_map
        .entry(section.clone())
        .or_insert_with(|| Value::Table(toml::Table::new()));
    let section_table = section_entry
        .as_table_mut()
        .with_context(|| format!("{}: [{}] must be a TOML table", path.display(), section))?;

    let parsed = parse_scalar(&value).context("parse value")?;
    section_table.insert(field.clone(), parsed);

    let out = toml::to_string(&root).context("serialize updated workspace config")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    std::fs::write(&path, out).with_context(|| format!("write {}", path.display()))?;

    glean_core::GleanConfig::load_merged(&workspace).with_context(|| {
        format!(
            "merged config invalid after set (check types and global config.toml): {}",
            path.display()
        )
    })?;

    eprintln!("Updated {} → [{}].{}", path.display(), section, field);
    Ok(())
}
