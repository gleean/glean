//! Cross-encoder reranking after hybrid / vector baseline.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::config::RerankConfig;
use crate::error::CoreError;
use crate::storage::StorageLayout;

const RERANK_TIMEOUT_MS: u64 = 1500;
/// Drop cached ONNX session after this idle period (see `reranking-strategy.md`).
const RERANK_SESSION_IDLE_SECS: u64 = 600;

/// Resolve `[rerank].model_path` relative to **`$GLEAN_STORAGE_ROOT`** (absolute paths unchanged).
pub fn resolve_rerank_model_path(layout: &StorageLayout, cfg: &RerankConfig) -> PathBuf {
    let p = Path::new(cfg.model_path.trim());
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        layout.root.join(p)
    }
}

/// `true` when `path` is a readable regular file (ONNX artifact present).
pub fn onnx_model_exists(path: &Path) -> bool {
    path.is_file()
}

/// Best-effort low-power detection; skips rerank when likely on battery saver.
///
/// **Platform note:** only macOS runs `pmset -g batt` today. Windows/Linux have no
/// equivalent probe in OSS; callers should treat this as a no-op off macOS.
fn low_power_mode_active() -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let out = Command::new("pmset")
            .args(["-g", "batt"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());
        if let Some(text) = out {
            let lower = text.to_ascii_lowercase();
            if lower.contains("low power mode") && lower.contains("on") {
                return true;
            }
        }
    }
    let _ = ();
    false
}

/// Optionally apply cross-encoder scores to `semantic_search` hits.
pub fn apply_cross_encoder_rerank(
    cfg: &RerankConfig,
    layout: &StorageLayout,
    query: &str,
    hits: Vec<(String, String)>,
) -> Result<Vec<(String, String)>, CoreError> {
    if !cfg.enabled || hits.is_empty() {
        return Ok(hits);
    }

    if low_power_mode_active() {
        tracing::debug!(
            target: "glean_rerank",
            "low-power heuristic active — returning baseline order"
        );
        return Ok(hits);
    }

    let model_path = resolve_rerank_model_path(layout, cfg);
    if !onnx_model_exists(&model_path) {
        tracing::warn!(
            target: "glean_rerank",
            path = %model_path.display(),
            cache_dir = %layout.reranker_cache_dir().display(),
            "rerank enabled but ONNX model file missing — returning baseline order"
        );
        return Ok(hits);
    }

    #[cfg(feature = "fastembed")]
    {
        let baseline = hits.clone();
        let query_owned = query.to_string();
        let cfg = cfg.clone();
        let model_path = model_path.clone();
        let cache_dir = layout.reranker_cache_dir();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = rerank_with_fastembed(&cfg, &model_path, &cache_dir, &query_owned, hits);
            let _ = tx.send(result);
        });
        match rx.recv_timeout(Duration::from_millis(RERANK_TIMEOUT_MS)) {
            Ok(Ok(reordered)) => Ok(reordered),
            Ok(Err(e)) => {
                tracing::warn!(
                    target: "glean_rerank",
                    error = %e,
                    "rerank inference failed — returning baseline order"
                );
                Ok(baseline)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                tracing::warn!(
                    target: "glean_rerank",
                    timeout_ms = RERANK_TIMEOUT_MS,
                    "rerank timed out — returning baseline order"
                );
                Ok(baseline)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err(CoreError::Msg("rerank worker thread panicked".into()))
            }
        }
    }

    #[cfg(not(feature = "fastembed"))]
    {
        tracing::debug!(
            target: "glean_rerank",
            top_k = cfg.top_k,
            model_path = %model_path.display(),
            hits = hits.len(),
            "fastembed feature disabled — returning baseline order"
        );
        Ok(hits)
    }
}

#[cfg(feature = "fastembed")]
fn rerank_with_fastembed(
    cfg: &RerankConfig,
    model_path: &Path,
    cache_dir: &Path,
    query: &str,
    hits: Vec<(String, String)>,
) -> Result<Vec<(String, String)>, CoreError> {
    use fastembed::RerankResult;

    let top_k = cfg.top_k.max(1) as usize;
    let rerank_n = top_k.min(hits.len());
    let (head, tail) = hits.split_at(rerank_n);
    let docs: Vec<&str> = head.iter().map(|(_, text)| text.as_str()).collect();

    let results: Vec<RerankResult> = with_reranker(model_path, cache_dir, |reranker| {
        reranker
            .rerank(query, docs.as_slice(), false, None)
            .map_err(|e| CoreError::Msg(format!("rerank inference: {e}")))
    })?;

    let mut scored: Vec<(usize, f32)> = results.into_iter().map(|r| (r.index, r.score)).collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut out = Vec::with_capacity(hits.len());
    for (idx, _) in scored {
        if let Some(pair) = head.get(idx) {
            out.push(pair.clone());
        }
    }
    for pair in tail {
        out.push(pair.clone());
    }
    Ok(out)
}

#[cfg(feature = "fastembed")]
struct RerankSession {
    reranker: fastembed::TextRerank,
    last_used: Instant,
}

#[cfg(feature = "fastembed")]
fn with_reranker<R>(
    model_path: &Path,
    cache_dir: &Path,
    f: impl FnOnce(&mut fastembed::TextRerank) -> Result<R, CoreError>,
) -> Result<R, CoreError> {
    static SLOT: OnceLock<Mutex<Option<RerankSession>>> = OnceLock::new();
    let mutex = SLOT.get_or_init(|| Mutex::new(None));
    let mut guard = mutex
        .lock()
        .map_err(|_| CoreError::Msg("reranker mutex poisoned".into()))?;
    let idle = Duration::from_secs(RERANK_SESSION_IDLE_SECS);
    if guard
        .as_ref()
        .is_some_and(|s| s.last_used.elapsed() >= idle)
    {
        tracing::debug!(
            target: "glean_rerank",
            idle_secs = RERANK_SESSION_IDLE_SECS,
            "dropping idle rerank session"
        );
        *guard = None;
    }
    if guard.is_none() {
        *guard = Some(RerankSession {
            reranker: load_reranker(model_path, cache_dir)?,
            last_used: Instant::now(),
        });
    }
    let session = guard.as_mut().expect("reranker slot");
    session.last_used = Instant::now();
    f(&mut session.reranker)
}

/// Pre-download BGE reranker assets into `layout.reranker_cache_dir()` (FastEmbed / HF hub).
#[cfg(feature = "fastembed")]
pub fn pull_bge_rerank_model(layout: &StorageLayout) -> Result<PathBuf, CoreError> {
    let cache_dir = layout.reranker_cache_dir();
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| CoreError::Msg(format!("mkdir rerank cache: {e}")))?;
    let dummy = layout.root.join("models/reranker/bge-v2-m3.onnx");
    let _ = load_reranker(&dummy, &cache_dir)?;
    Ok(cache_dir)
}

#[cfg(not(feature = "fastembed"))]
pub fn pull_bge_rerank_model(_layout: &StorageLayout) -> Result<PathBuf, CoreError> {
    Err(CoreError::Msg(
        "rerank model pull requires glean-core built with feature `fastembed`".into(),
    ))
}

#[cfg(feature = "fastembed")]
fn load_reranker(model_path: &Path, cache_dir: &Path) -> Result<fastembed::TextRerank, CoreError> {
    use fastembed::{RerankInitOptions, RerankerModel, TextRerank};

    let eps = execution_providers_for_rerank();

    if model_path.is_file() {
        if let Ok(reranker) = try_load_user_onnx(model_path, &eps) {
            return Ok(reranker);
        }
        tracing::warn!(
            target: "glean_rerank",
            path = %model_path.display(),
            "could not load ONNX with sidecar tokenizer files — falling back to built-in BGE download"
        );
    }

    std::fs::create_dir_all(cache_dir)
        .map_err(|e| CoreError::Msg(format!("mkdir rerank cache: {e}")))?;

    let opts = RerankInitOptions::new(RerankerModel::BGERerankerV2M3)
        .with_execution_providers(eps)
        .with_cache_dir(cache_dir.to_path_buf())
        .with_show_download_progress(false);
    TextRerank::try_new(opts).map_err(|e| CoreError::Msg(format!("load BGE reranker: {e}")))
}

#[cfg(feature = "fastembed")]
fn try_load_user_onnx(
    model_path: &Path,
    eps: &[ort::execution_providers::ExecutionProviderDispatch],
) -> Result<fastembed::TextRerank, CoreError> {
    use fastembed::{
        OnnxSource, RerankInitOptionsUserDefined, TextRerank, TokenizerFiles,
        UserDefinedRerankingModel,
    };

    let parent = model_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let read = |name: &str| -> Result<Vec<u8>, CoreError> {
        let p = parent.join(name);
        std::fs::read(&p)
            .map_err(|e| CoreError::Msg(format!("read reranker tokenizer {}: {e}", p.display())))
    };

    let tokenizer_files = TokenizerFiles {
        tokenizer_file: read("tokenizer.json")?,
        config_file: read("config.json")?,
        special_tokens_map_file: read("special_tokens_map.json")?,
        tokenizer_config_file: read("tokenizer_config.json")?,
    };

    let model =
        UserDefinedRerankingModel::new(OnnxSource::File(model_path.to_path_buf()), tokenizer_files);
    let mut opts = RerankInitOptionsUserDefined::default();
    opts.execution_providers = eps.to_vec();
    opts.max_length = 512;
    TextRerank::try_new_from_user_defined(model, opts)
        .map_err(|e| CoreError::Msg(format!("load reranker from {}: {e}", model_path.display())))
}

#[cfg(feature = "fastembed")]
fn execution_providers_for_rerank() -> Vec<ort::execution_providers::ExecutionProviderDispatch> {
    #[cfg(target_os = "macos")]
    {
        use ort::ep::{CoreML, CPU};
        vec![CoreML::default().build(), CPU::default().build()]
    }
    #[cfg(not(target_os = "macos"))]
    {
        use ort::ep::CPU;
        vec![CPU::default().build()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RerankConfig;
    use crate::storage::StorageLayout;

    fn sample_hits() -> Vec<(String, String)> {
        vec![("a".into(), "t1".into()), ("b".into(), "t2".into())]
    }

    #[test]
    fn resolve_model_path_relative_to_storage_root() {
        let layout = StorageLayout::from_root("/tmp/glean");
        let cfg = RerankConfig::default();
        let p = resolve_rerank_model_path(&layout, &cfg);
        assert_eq!(
            p,
            PathBuf::from("/tmp/glean/models/reranker/bge-v2-m3.onnx")
        );
    }

    #[test]
    fn when_disabled_returns_hits_untouched() {
        let layout = StorageLayout::from_root("/tmp");
        let cfg = RerankConfig {
            enabled: false,
            ..Default::default()
        };
        let hits = sample_hits();
        let out = apply_cross_encoder_rerank(&cfg, &layout, "query", hits).unwrap();
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn when_enabled_missing_model_returns_baseline() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = StorageLayout::from_root(tmp.path());
        let cfg = RerankConfig {
            enabled: true,
            ..Default::default()
        };
        let expected = sample_hits();
        let out = apply_cross_encoder_rerank(&cfg, &layout, "q", expected.clone()).unwrap();
        assert_eq!(out, expected);
    }
}
