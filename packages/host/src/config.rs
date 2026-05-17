//! Global `config.toml` editor (init / set / list helpers).

use std::path::PathBuf;

use glean_core::{ConfigLayer, GleanConfig, GlobalLayout};
use toml::Value;

use crate::HostError;

const CONFIG_INIT_HEADER: &str = r#"# Glean TOML (merge order: defaults → $GLEAN_STORAGE_ROOT/config.toml).
# See repository README for environment variables.
"#;

fn parse_key_path(key: &str) -> Result<(String, String), HostError> {
    let parts: Vec<&str> = key.trim().split('.').filter(|s| !s.is_empty()).collect();
    if parts.len() != 2 {
        return Err(HostError::msg(
            "KEY must be SECTION.FIELD (exactly one dot), e.g. rerank.enabled, embedding.device",
        ));
    }
    Ok((parts[0].to_ascii_lowercase(), parts[1].to_ascii_lowercase()))
}

fn validate_key(section: &str, field: &str) -> Result<(), HostError> {
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
        return Err(HostError::msg(format!(
            "unknown key `{section}.{field}`; allowed keys match `GleanConfig` (see `glean config list`)"
        )));
    }
    Ok(())
}

fn parse_scalar(raw: &str) -> Result<Value, HostError> {
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

/// Provenance comment block for `glean config list --show-sources`.
pub fn format_section_provenance(global: &GlobalLayout) -> Result<String, HostError> {
    let prov = GleanConfig::section_provenance(global)?;
    let mut lines = vec![
        "# Glean effective configuration".to_string(),
        "# Section provenance (later overrides earlier: default < global)".to_string(),
    ];
    for section in ["core", "indexing", "embedding", "rerank", "log"] {
        let layer = prov.get(section).copied().unwrap_or(ConfigLayer::Default);
        lines.push(format!("# [{section}] source: {}", layer.as_str()));
    }
    Ok(lines.join("\n"))
}

/// Load merged config from the current global layout env.
pub fn load_merged_config() -> Result<GleanConfig, HostError> {
    Ok(GleanConfig::load_merged()?)
}

/// Load merged config and serialize as TOML string.
pub fn merged_config_toml() -> Result<String, HostError> {
    let global = GlobalLayout::from_env_or_default()?;
    let cfg = GleanConfig::load_merged_with_global(&global)?;
    Ok(toml::to_string(&cfg)?)
}

/// Initialize `$GLEAN_STORAGE_ROOT/config.toml` from defaults (`force` overwrites).
pub fn init_global_config(force: bool) -> Result<PathBuf, HostError> {
    let global = GlobalLayout::from_env_or_default()?;
    let path = global.global_config_path();

    if path.is_file() && !force {
        return Err(HostError::msg(format!(
            "refusing to overwrite {}; pass --force to replace",
            path.display()
        )));
    }

    let body = default_config_template()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, body.as_bytes())?;

    GleanConfig::load_merged_with_global(&global).map_err(|e| {
        HostError::msg(format!(
            "written file parses, but merged load failed: {}: {e}",
            path.display()
        ))
    })?;

    Ok(path)
}

fn default_config_template() -> Result<String, HostError> {
    let cfg = GleanConfig::default();
    let toml_body = toml::to_string(&cfg)?;
    Ok(format!("{CONFIG_INIT_HEADER}\n{toml_body}"))
}

/// Set one scalar in global `config.toml`.
pub fn set_global_key(key: String, value: String) -> Result<PathBuf, HostError> {
    let (section, field) = parse_key_path(&key)?;
    validate_key(&section, &field)?;

    let global = GlobalLayout::from_env_or_default()?;
    let path = global.global_config_path();
    let mut root = if path.is_file() {
        let s = std::fs::read_to_string(&path)?;
        let v: Value = toml::from_str(&s)?;
        match v {
            Value::Table(_) => v,
            _ => {
                return Err(HostError::msg(format!(
                    "{}: root must be a TOML table",
                    path.display()
                )));
            }
        }
    } else {
        Value::Table(toml::Table::new())
    };

    let root_map = root.as_table_mut().expect("root is table");
    let section_entry = root_map
        .entry(section.clone())
        .or_insert_with(|| Value::Table(toml::Table::new()));
    let section_table = section_entry.as_table_mut().ok_or_else(|| {
        HostError::msg(format!(
            "{}: [{}] must be a TOML table",
            path.display(),
            section
        ))
    })?;

    let parsed = parse_scalar(&value)?;
    section_table.insert(field.clone(), parsed);

    let out = toml::to_string(&root)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, out)?;

    GleanConfig::load_merged_with_global(&global).map_err(|e| {
        HostError::msg(format!(
            "merged config invalid after set (check types): {}: {e}",
            path.display()
        ))
    })?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_template_contains_core_sections() {
        let body = default_config_template().unwrap();
        assert!(body.contains("[embedding]"));
        assert!(body.contains("[indexing]"));
    }
}
