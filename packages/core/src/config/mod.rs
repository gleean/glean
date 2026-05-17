//! Merged runtime configuration from defaults, storage-root `config.toml`,
//! and workspace `.glean/config.toml` (see internal `configuration-system.md` design).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml::map::Map;
use toml::Value;

use crate::error::CoreError;
use crate::storage::StorageLayout;

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
    Workspace,
}

impl ConfigLayer {
    /// Stable label for CLI comments and tests.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Global => "global",
            Self::Workspace => "workspace",
        }
    }
}

const CONFIG_SECTIONS: &[&str] = &["core", "indexing", "embedding", "rerank", "log"];

fn read_config_root_table(path: &Path) -> Result<Option<Map<String, Value>>, CoreError> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).map_err(|e| CoreError::InvalidConfigToml {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    let value: Value = toml::from_str(&text).map_err(|e| CoreError::InvalidConfigToml {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    match value {
        Value::Table(t) => Ok(Some(t)),
        _ => Err(CoreError::InvalidConfigToml {
            path: path.to_path_buf(),
            message: "root must be a TOML table".into(),
        }),
    }
}

fn section_layer_from_tables(
    global: Option<&Map<String, Value>>,
    workspace: Option<&Map<String, Value>>,
    section: &str,
) -> ConfigLayer {
    if workspace.is_some_and(|m| m.contains_key(section)) {
        ConfigLayer::Workspace
    } else if global.is_some_and(|m| m.contains_key(section)) {
        ConfigLayer::Global
    } else {
        ConfigLayer::Default
    }
}

impl GleanConfig {
    /// Section-level provenance for `glean config list --show-sources`.
    pub fn section_provenance(
        workspace_root: &Path,
        layout: &StorageLayout,
    ) -> Result<HashMap<String, ConfigLayer>, CoreError> {
        let global_path = layout.root.join("config.toml");
        let workspace_path = workspace_root.join(".glean").join("config.toml");
        let global = read_config_root_table(&global_path)?;
        let workspace = read_config_root_table(&workspace_path)?;
        let mut out = HashMap::new();
        for &section in CONFIG_SECTIONS {
            out.insert(
                section.to_string(),
                section_layer_from_tables(global.as_ref(), workspace.as_ref(), section),
            );
        }
        Ok(out)
    }

    /// Paths checked for provenance and daemon hot-reload (global then workspace).
    pub fn config_watch_paths(
        workspace_root: &Path,
        layout: &StorageLayout,
    ) -> (PathBuf, PathBuf) {
        (
            layout.root.join("config.toml"),
            workspace_root.join(".glean").join("config.toml"),
        )
    }

    /// Merge defaults with `layout.root/config.toml` then `<workspace_root>/.glean/config.toml`.
    /// Later tables override earlier keys at each section level.
    pub fn load_merged_with_layout(
        workspace_root: &Path,
        layout: &StorageLayout,
    ) -> Result<Self, CoreError> {
        let mut merged = Map::new();

        let global_path = layout.root.join("config.toml");
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

        let local_path = workspace_root.join(".glean").join("config.toml");
        if local_path.is_file() {
            let text =
                fs::read_to_string(&local_path).map_err(|e| CoreError::InvalidConfigToml {
                    path: local_path.clone(),
                    message: e.to_string(),
                })?;
            let value: Value = toml::from_str(&text).map_err(|e| CoreError::InvalidConfigToml {
                path: local_path.clone(),
                message: e.to_string(),
            })?;
            if let Value::Table(t) = value {
                merge_maps(&mut merged, t);
            } else {
                return Err(CoreError::InvalidConfigToml {
                    path: local_path,
                    message: "root must be a TOML table".into(),
                });
            }
        }

        let wrapped = Value::Table(merged);
        let serialized = toml::to_string(&wrapped)
            .map_err(|e| CoreError::Msg(format!("config serialize: {e}")))?;
        toml::from_str(&serialized).map_err(|e| CoreError::InvalidConfigToml {
            path: workspace_root.to_path_buf(),
            message: e.to_string(),
        })
    }

    /// Merge defaults with `$GLEAN_STORAGE_ROOT/config.toml` then `<workspace_root>/.glean/config.toml`
    /// (later files override earlier keys at the TOML table level).
    ///
    /// Does not read `GLEAN_LOG` here; the CLI applies log filters separately.
    pub fn load_merged(workspace_root: &Path) -> Result<Self, CoreError> {
        let layout = StorageLayout::from_env_or_default()?;
        Self::load_merged_with_layout(workspace_root, &layout)
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
    fn merged_workspace_overrides_global_rerank() {
        let tmp = tempdir().unwrap();
        let storage = tmp.path().join("storage");
        fs::create_dir_all(&storage).unwrap();
        let layout = StorageLayout::from_root(&storage);

        fs::write(
            storage.join("config.toml"),
            r#"
[rerank]
enabled = false
top_k = 10
"#,
        )
        .unwrap();

        let ws = tmp.path().join("ws");
        fs::create_dir_all(ws.join(".glean")).unwrap();
        fs::write(
            ws.join(".glean").join("config.toml"),
            r#"
[rerank]
enabled = true
"#,
        )
        .unwrap();

        let cfg = GleanConfig::load_merged_with_layout(&ws, &layout).expect("load merged");
        assert!(cfg.rerank.enabled);
        assert_eq!(cfg.rerank.top_k, 10);
    }

    #[test]
    fn global_only_applies_when_no_workspace_section() {
        let tmp = tempdir().unwrap();
        let storage = tmp.path().join("glean_home");
        fs::create_dir_all(&storage).unwrap();
        let layout = StorageLayout::from_root(&storage);

        fs::write(
            storage.join("config.toml"),
            r#"
[log]
level = "debug"
"#,
        )
        .unwrap();

        let ws = tmp.path().join("empty_ws");
        fs::create_dir_all(&ws).unwrap();

        let cfg = GleanConfig::load_merged_with_layout(&ws, &layout).expect("load merged");
        assert_eq!(cfg.log.level, "debug");
    }

    #[test]
    fn no_config_files_yields_defaults() {
        let tmp = tempdir().unwrap();
        let storage = tmp.path().join("store");
        fs::create_dir_all(&storage).unwrap();
        let layout = StorageLayout::from_root(&storage);

        let ws = tmp.path().join("ws");
        fs::create_dir_all(&ws).unwrap();

        let cfg = GleanConfig::load_merged_with_layout(&ws, &layout).expect("load merged");
        assert_eq!(cfg.log.level, default_log_level());
        assert!(cfg.indexing.use_gitignore);
        assert_eq!(cfg.indexing.watch_interval, default_watch_interval());
        assert_eq!(cfg.embedding.dimension, default_embed_dim());
    }

    #[test]
    fn invalid_workspace_config_returns_error() {
        let tmp = tempdir().unwrap();
        let storage = tmp.path().join("store");
        fs::create_dir_all(&storage).unwrap();
        let layout = StorageLayout::from_root(&storage);

        let ws = tmp.path().join("bad_ws");
        fs::create_dir_all(ws.join(".glean")).unwrap();
        fs::write(ws.join(".glean/config.toml"), "not-valid-toml [[[").unwrap();

        let err = GleanConfig::load_merged_with_layout(&ws, &layout).unwrap_err();
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
    fn section_provenance_marks_workspace_rerank() {
        let tmp = tempdir().unwrap();
        let storage = tmp.path().join("s");
        fs::create_dir_all(&storage).unwrap();
        let layout = StorageLayout::from_root(&storage);

        fs::write(
            storage.join("config.toml"),
            r#"
[rerank]
enabled = false
"#,
        )
        .unwrap();

        let ws = tmp.path().join("w");
        fs::create_dir_all(ws.join(".glean")).unwrap();
        fs::write(
            ws.join(".glean/config.toml"),
            r#"
[rerank]
enabled = true
"#,
        )
        .unwrap();

        let prov = GleanConfig::section_provenance(&ws, &layout).expect("provenance");
        assert_eq!(prov.get("rerank").copied(), Some(ConfigLayer::Workspace));
        assert_eq!(prov.get("core").copied(), Some(ConfigLayer::Default));
    }

    #[test]
    fn nested_core_threads_merge_not_replaced_whole_section() {
        let tmp = tempdir().unwrap();
        let storage = tmp.path().join("s");
        fs::create_dir_all(&storage).unwrap();
        let layout = StorageLayout::from_root(&storage);

        fs::write(
            storage.join("config.toml"),
            r#"
[core]
path = "global_path"
threads = 4
"#,
        )
        .unwrap();

        let ws = tmp.path().join("w");
        fs::create_dir_all(ws.join(".glean")).unwrap();
        fs::write(
            ws.join(".glean/config.toml"),
            r#"
[core]
threads = 1
"#,
        )
        .unwrap();

        let cfg = GleanConfig::load_merged_with_layout(&ws, &layout).expect("load merged");
        assert_eq!(cfg.core.path, "global_path");
        assert_eq!(cfg.core.threads, 1);
    }
}
