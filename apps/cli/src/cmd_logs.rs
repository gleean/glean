//! Human-facing `glean logs`: tail the newest rolling log file under the storage layout.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Read up to `max_lines` lines from the end of a text file (loads whole file; OK for local logs).
fn tail_file(path: &Path, max_lines: usize) -> Result<Vec<String>> {
    let f = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = BufReader::new(f);
    let mut dq = VecDeque::with_capacity(max_lines.max(1));
    for line in reader.lines() {
        let line = line?;
        if dq.len() >= max_lines {
            dq.pop_front();
        }
        dq.push_back(line);
    }
    Ok(dq.into_iter().collect())
}

/// List log files under `logs_dir`, newest by `modified` time, optionally filtered by filename prefix.
fn sorted_log_files(logs_dir: &Path, prefix: Option<&str>) -> Result<Vec<PathBuf>> {
    let mut entries = Vec::new();
    for ent in
        std::fs::read_dir(logs_dir).with_context(|| format!("read_dir {}", logs_dir.display()))?
    {
        let ent = ent?;
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if let Some(pre) = prefix {
            if !name.starts_with(pre) {
                continue;
            }
        }
        let meta = ent.metadata()?;
        let modified = meta.modified().ok();
        entries.push((modified, p));
    }
    entries.sort_by_key(|e| std::cmp::Reverse(e.0));
    Ok(entries.into_iter().map(|(_, p)| p).collect())
}

/// Matches rolling file prefixes used in `logging.rs` (`daily` appender).
#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub enum LogRuntimeFilter {
    /// `cli*` (MCP / status)
    #[default]
    Cli,
    /// `daemon*`
    Daemon,
    /// Newest log file regardless of prefix
    All,
}

/// Entry point for `glean logs`.
pub fn run_logs(source: LogRuntimeFilter, lines: usize) -> Result<()> {
    let layout =
        glean_core::StorageLayout::from_env_or_default().context("resolve GLEAN_STORAGE_ROOT")?;
    let log_dir = layout.root.join("logs");
    if !log_dir.is_dir() {
        anyhow::bail!(
            "log directory does not exist: {} (run daemon or mcp once to create logs)",
            log_dir.display()
        );
    }

    let prefix = match source {
        LogRuntimeFilter::Cli => Some("cli"),
        LogRuntimeFilter::Daemon => Some("daemon"),
        LogRuntimeFilter::All => None,
    };

    let files = sorted_log_files(&log_dir, prefix)?;
    if files.is_empty() {
        anyhow::bail!("no matching log files under {}", log_dir.display());
    }

    let target = &files[0];
    let tail = tail_file(target, lines)?;
    println!("{} (last {} lines)", target.display(), lines);
    for line in tail {
        println!("{line}");
    }
    Ok(())
}
