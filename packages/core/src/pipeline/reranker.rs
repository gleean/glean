//! Cross-encoder reranking after hybrid / vector baseline.
//!
//! ONNX Runtime session, model download, timeout, and battery guardrails are **not** wired yet.
//! When `[rerank].enabled` is true, this module logs and returns hits in **baseline order**.

use crate::config::RerankConfig;
use crate::error::CoreError;

/// Optionally apply cross-encoder scores to `semantic_search` hits.
///
/// **Current behavior**: no-op reorder; preserves `lance_chunks::semantic_search_chunks` order.
pub fn apply_cross_encoder_rerank(
    cfg: &RerankConfig,
    query: &str,
    hits: Vec<(String, String)>,
) -> Result<Vec<(String, String)>, CoreError> {
    if !cfg.enabled || hits.is_empty() {
        return Ok(hits);
    }

    tracing::debug!(
        target: "glean_rerank",
        top_k = cfg.top_k,
        model_path = %cfg.model_path,
        hits = hits.len(),
        query_len = query.len(),
        "cross-encoder rerank enabled; ONNX reorder not implemented — returning baseline order"
    );

    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RerankConfig;

    fn sample_hits() -> Vec<(String, String)> {
        vec![("a".into(), "t1".into()), ("b".into(), "t2".into())]
    }

    #[test]
    fn when_disabled_returns_hits_untouched() {
        let cfg = RerankConfig {
            enabled: false,
            ..Default::default()
        };
        let hits = sample_hits();
        let out = apply_cross_encoder_rerank(&cfg, "query", hits).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, "a");
        assert_eq!(out[1].0, "b");
    }

    #[test]
    fn when_enabled_noop_keeps_input_order() {
        let cfg = RerankConfig {
            enabled: true,
            top_k: 5,
            ..Default::default()
        };
        let expected = sample_hits();
        let out = apply_cross_encoder_rerank(&cfg, "needle", expected.clone()).unwrap();
        assert_eq!(out, expected);
    }

    #[test]
    fn empty_hits_short_circuits_without_error() {
        let cfg = RerankConfig {
            enabled: true,
            ..Default::default()
        };
        let out = apply_cross_encoder_rerank(&cfg, "", vec![]).unwrap();
        assert!(out.is_empty());
    }
}
