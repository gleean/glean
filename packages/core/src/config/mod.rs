//! Merged runtime configuration from defaults and `$GLEAN_STORAGE_ROOT/config.toml`.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use toml::map::Map;
use toml::Value;

use crate::error::CoreError;
use crate::storage::GlobalLayout;

/// Root config document (TOML sections map to these structs).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GleanConfig {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub indexing: IndexingConfig,
    #[serde(default)]
    pub embedding: EmbeddingConfig,
    #[serde(default)]
    pub rerank: RerankConfig,
    #[serde(default)]
    pub log: LogConfig,
}

#[allow(clippy::derivable_impls)] // explicit defaults mirror design TOML; nested defaults are non-trivial
impl Default for GleanConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig::default(),
            indexing: IndexingConfig::default(),
            embedding: EmbeddingConfig::default(),
            rerank: RerankConfig::default(),
            log: LogConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreConfig {
    #[serde(default = "default_core_path")]
    pub path: String,
    #[serde(default)]
    pub threads: u32,
}

fn default_core_path() -> String {
    ".".to_string()
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            path: default_core_path(),
            threads: 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexingConfig {
    #[serde(default = "default_watch_interval")]
    pub watch_interval: u64,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
    #[serde(default = "default_true")]
    pub use_gitignore: bool,
}

fn default_watch_interval() -> u64 {
    10
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024
}

fn default_true() -> bool {
    true
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            watch_interval: default_watch_interval(),
            max_file_size: default_max_file_size(),
            use_gitignore: true,
        }
    }
}

impl IndexingConfig {
    /// `(min, max)` byte bounds for workspace scan / incremental sync.
    pub fn sync_byte_limits(&self) -> (u64, u64) {
        use crate::pipeline::{DEFAULT_MAX_FILE_BYTES, DEFAULT_MIN_FILE_BYTES};
        let max = if self.max_file_size == 0 {
            DEFAULT_MAX_FILE_BYTES
        } else {
            self.max_file_size
        };
        (DEFAULT_MIN_FILE_BYTES, max)
    }

    /// Daemon poll interval after filesystem notify; `None` when `watch_interval == 0` (initial sync only).
    pub fn watch_poll_interval(&self) -> Option<std::time::Duration> {
        if self.watch_interval == 0 {
            None
        } else {
            Some(std::time::Duration::from_secs(self.watch_interval.max(1)))
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EmbeddingConfig {
    #[serde(default = "default_embed_model")]
    pub model: String,
    #[serde(default = "default_embed_dim")]
    pub dimension: u32,
    #[serde(default = "default_device")]
    pub device: String,
}

fn default_embed_model() -> String {
    "all-MiniLM-L6-v2".to_string()
}

fn default_embed_dim() -> u32 {
    384
}

fn default_device() -> String {
    "cpu".to_string()
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: default_embed_model(),
            dimension: default_embed_dim(),
            device: default_device(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RerankConfig {
    /// Cross-encoder rerank is opt-in; default stays off until implemented end-to-end.
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_rerank_top_k")]
    pub top_k: u32,
    #[serde(default = "default_rerank_model_path")]
    pub model_path: String,
}

fn default_rerank_top_k() -> u32 {
    20
}

fn default_rerank_model_path() -> String {
    "models/reranker/bge-v2-m3.onnx".to_string()
}

impl Default for RerankConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            top_k: default_rerank_top_k(),
            model_path: default_rerank_model_path(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_true")]
    pub mask_privacy: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            mask_privacy: true,
        }
    }
}

/// Which merge layer owns a TOML section in the effective config (section-level, not per-field).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigLayer {
    Default,
    Global,
}

impl ConfigLayer {
    /// Stable label for CLI comments and tests.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Global => "global",
        }
    }
}

const CONFIG_SECTIONS: &[&str] = &["core", "indexing", "embedding", "rerank", "log"];

fn read_config_root_table(path: &PathBuf) -> Result<Option<Map<String, Value>>, CoreError> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).map_err(|e| CoreError::InvalidConfigToml {
        path: path.clone(),
        message: e.to_string(),
    })?;
    let value: Value = toml::from_str(&text).map_err(|e| CoreError::InvalidConfigToml {
        path: path.clone(),
        message: e.to_string(),
    })?;
    match value {
        Value::Table(t) => Ok(Some(t)),
        _ => Err(CoreError::InvalidConfigToml {
            path: path.clone(),
            message: "root must be a TOML table".into(),
        }),
    }
}

fn section_layer_from_global(global: Option<&Map<String, Value>>, section: &str) -> ConfigLayer {
    if global.is_some_and(|m| m.contains_key(section)) {
        ConfigLayer::Global
    } else {
        ConfigLayer::Default
    }
}

impl GleanConfig {
    /// Section-level provenance for `glean config list --show-sources`.
    pub fn section_provenance(
        global: &GlobalLayout,
    ) -> Result<HashMap<String, ConfigLayer>, CoreError> {
        let global_path = global.global_config_path();
        let global_table = read_config_root_table(&global_path)?;
        let mut out = HashMap::new();
        for &section in CONFIG_SECTIONS {
            out.insert(
                section.to_string(),
                section_layer_from_global(global_table.as_ref(), section),
            );
        }
        Ok(out)
    }

    /// Path watched for daemon config hot-reload.
    pub fn global_config_watch_path(global: &GlobalLayout) -> PathBuf {
        global.global_config_path()
    }

    /// Merge defaults with `$GLEAN_STORAGE_ROOT/config.toml` (section-level recursive merge).
    pub fn load_merged_with_global(global: &GlobalLayout) -> Result<Self, CoreError> {
        let mut merged = Map::new();

        let global_path = global.global_config_path();
        if global_path.is_file() {
            let text =
                fs::read_to_string(&global_path).map_err(|e| CoreError::InvalidConfigToml {
                    path: global_path.clone(),
                    message: e.to_string(),
                })?;
            let value: Value = toml::from_str(&text).map_err(|e| CoreError::InvalidConfigToml {
                path: global_path.clone(),
                message: e.to_string(),
            })?;
            if let Value::Table(t) = value {
                merge_maps(&mut merged, t);
            } else {
                return Err(CoreError::InvalidConfigToml {
                    path: global_path,
                    message: "root must be a TOML table".into(),
                });
            }
        }

        let wrapped = Value::Table(merged);
        let serialized = toml::to_string(&wrapped)
            .map_err(|e| CoreError::Msg(format!("config serialize: {e}")))?;
        toml::from_str(&serialized).map_err(|e| CoreError::InvalidConfigToml {
            path: global_path,
            message: e.to_string(),
        })
    }

    /// Merge defaults with `$GLEAN_STORAGE_ROOT/config.toml`.
    ///
    /// Does not read `GLEAN_LOG` here; the CLI applies log filters separately.
    pub fn load_merged() -> Result<Self, CoreError> {
        let global = GlobalLayout::from_env_or_default()?;
        Self::load_merged_with_global(&global)
    }
}

fn merge_maps(base: &mut Map<String, Value>, overlay: Map<String, Value>) {
    for (k, v) in overlay {
        match base.get_mut(&k) {
            Some(Value::Table(bt)) if v.as_table().is_some() => {
                if let Value::Table(ot) = v {
                    merge_maps(bt, ot);
                }
            }
            _ => {
                base.insert(k, v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn global_overrides_default_rerank_enabled() {
        let tmp = tempdir().unwrap();
        let global = GlobalLayout::from_root(tmp.path());

        fs::write(
            tmp.path().join("config.toml"),
            r#"
[rerank]
enabled = true
top_k = 10
"#,
        )
        .unwrap();

        let cfg = GleanConfig::load_merged_with_global(&global).expect("load merged");
        assert!(cfg.rerank.enabled);
        assert_eq!(cfg.rerank.top_k, 10);
    }

    #[test]
    fn global_log_level_applies() {
        let tmp = tempdir().unwrap();
        let global = GlobalLayout::from_root(tmp.path());

        fs::write(
            tmp.path().join("config.toml"),
            r#"
[log]
level = "debug"
"#,
        )
        .unwrap();

        let cfg = GleanConfig::load_merged_with_global(&global).expect("load merged");
        assert_eq!(cfg.log.level, "debug");
    }

    #[test]
    fn no_config_files_yields_defaults() {
        let tmp = tempdir().unwrap();
        let global = GlobalLayout::from_root(tmp.path());

        let cfg = GleanConfig::load_merged_with_global(&global).expect("load merged");
        assert_eq!(cfg.log.level, default_log_level());
        assert!(cfg.indexing.use_gitignore);
        assert_eq!(cfg.indexing.watch_interval, default_watch_interval());
        assert_eq!(cfg.embedding.dimension, default_embed_dim());
    }

    #[test]
    fn invalid_global_config_returns_error() {
        let tmp = tempdir().unwrap();
        let global = GlobalLayout::from_root(tmp.path());
        fs::write(tmp.path().join("config.toml"), "not-valid-toml [[[").unwrap();

        let err = GleanConfig::load_merged_with_global(&global).unwrap_err();
        match err {
            CoreError::InvalidConfigToml { path, .. } => {
                assert!(
                    path.ends_with("config.toml"),
                    "unexpected path {}",
                    path.display()
                )
            }
            other => panic!("expected InvalidConfigToml, got {other:?}"),
        }
    }

    #[test]
    fn indexing_sync_byte_limits_uses_config_max() {
        let cfg = IndexingConfig {
            max_file_size: 2_000_000,
            ..Default::default()
        };
        let (_, max) = cfg.sync_byte_limits();
        assert_eq!(max, 2_000_000);
    }

    #[test]
    fn watch_poll_interval_zero_disables_tick() {
        let mut cfg = IndexingConfig {
            watch_interval: 0,
            ..Default::default()
        };
        assert!(cfg.watch_poll_interval().is_none());
        cfg.watch_interval = 5;
        assert_eq!(cfg.watch_poll_interval().unwrap().as_secs(), 5);
    }

    #[test]
    fn section_provenance_marks_global_rerank() {
        let tmp = tempdir().unwrap();
        let global = GlobalLayout::from_root(tmp.path());

        fs::write(
            tmp.path().join("config.toml"),
            r#"
[rerank]
enabled = false
"#,
        )
        .unwrap();

        let prov = GleanConfig::section_provenance(&global).expect("provenance");
        assert_eq!(prov.get("rerank").copied(), Some(ConfigLayer::Global));
        assert_eq!(prov.get("core").copied(), Some(ConfigLayer::Default));
    }

    #[test]
    fn nested_core_threads_merge_not_replaced_whole_section() {
        let tmp = tempdir().unwrap();
        let global = GlobalLayout::from_root(tmp.path());

        fs::write(
            tmp.path().join("config.toml"),
            r#"
[core]
path = "global_path"
threads = 4
"#,
        )
        .unwrap();

        let cfg = GleanConfig::load_merged_with_global(&global).expect("load merged");
        assert_eq!(cfg.core.path, "global_path");
        assert_eq!(cfg.core.threads, 4);
    }
}
