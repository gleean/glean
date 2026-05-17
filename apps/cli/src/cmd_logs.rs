//! Human-facing `glean logs`: tail the newest rolling log file under the storage layout.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

/// Poll interval when waiting for more bytes at EOF.
const FOLLOW_POLL_MS: u64 = 200;
/// How often to check for a newer rolling log file (`mtime` / filename).
const ROTATE_CHECK_INTERVAL: u32 = 25;

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

/// Print the last `tail_lines` lines, then stream new lines until a newer log file appears.
/// Returns the path of that newer file so the caller can continue following it.
fn tail_then_follow_until_newer_file(
    path: &Path,
    tail_lines: usize,
    logs_dir: &Path,
    prefix: Option<&str>,
) -> Result<PathBuf> {
    let mut reader =
        BufReader::new(File::open(path).with_context(|| format!("open {}", path.display()))?);

    if tail_lines > 0 {
        let mut dq = VecDeque::with_capacity(tail_lines.max(1));
        for line in reader.by_ref().lines() {
            let line = line?;
            if dq.len() >= tail_lines {
                dq.pop_front();
            }
            dq.push_back(line);
        }
        for line in dq {
            println!("{line}");
        }
    } else {
        reader.seek(SeekFrom::End(0))?;
    }

    let mut poll_ticks: u32 = 0;
    let mut buf = String::new();
    loop {
        buf.clear();
        let n = reader.read_line(&mut buf)?;
        if n > 0 {
            print!("{buf}");
            std::io::stdout().flush()?;
            poll_ticks = 0;
            continue;
        }

        thread::sleep(Duration::from_millis(FOLLOW_POLL_MS));
        poll_ticks += 1;
        if poll_ticks < ROTATE_CHECK_INTERVAL {
            continue;
        }
        poll_ticks = 0;

        let files = sorted_log_files(logs_dir, prefix)?;
        let Some(newest) = files.first() else {
            continue;
        };
        if newest != path {
            return Ok(newest.clone());
        }
    }
}

/// Entry point for `glean logs`.
pub fn run_logs(source: LogRuntimeFilter, lines: usize, follow: bool) -> Result<()> {
    let layout =
        glean_core::GlobalLayout::from_env_or_default().context("resolve GLEAN_STORAGE_ROOT")?;
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

    let mut path = files.into_iter().next().expect("non-empty files");
    print!("{} (last {} lines)", path.display(), lines);
    if follow {
        print!(" — following (Ctrl+C to stop)");
    }
    println!();

    if !follow {
        for line in tail_file(&path, lines)? {
            println!("{line}");
        }
        return Ok(());
    }

    let mut tail_n = lines;
    loop {
        let next = tail_then_follow_until_newer_file(&path, tail_n, &log_dir, prefix)?;
        if next != path {
            eprintln!("glean logs: switched to {}", next.display());
        }
        path = next;
        tail_n = 0;
    }
}
