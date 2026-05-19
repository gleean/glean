//! Workspace-wide inclusion rules that do not depend on file type registration.

use std::path::Path;

/// Skip known noisy or huge directory segments (exact name match on a normal path component).
const SKIPPED_DIR_NAMES: &[&str] = &[
    ".git",
    ".idea",
    ".svn",
    ".hg",
    "node_modules",
    "target",
    "vendor",
    "venv",
    "__pycache__",
    "tmp",
    "System Volume Information",
];

/// Default lower bound for indexing (very small files carry no useful embedding signal).
pub const DEFAULT_MIN_FILE_BYTES: u64 = 10;

/// Never index these extensions at the global layer (security), even if a parser were registered.
const BLOCKED_EXECUTABLE_EXTENSIONS: &[&str] = &[
    "exe", "dll", "so", "dylib", "bin", "app", "msi", "deb", "rpm",
];

/// True when `ext` (lowercase, no dot) is force-blocked before Dispatcher.
pub fn is_blocked_executable_extension(ext: &str) -> bool {
    BLOCKED_EXECUTABLE_EXTENSIONS
        .iter()
        .any(|b| ext.eq_ignore_ascii_case(b))
}

/// `true` when this relative path should never be traversed for workspace indexing.
pub fn should_skip_path_components(rel: &Path) -> bool {
    rel.components().any(|c| {
        if let std::path::Component::Normal(name) = c {
            let name = name.to_string_lossy();
            SKIPPED_DIR_NAMES
                .iter()
                .any(|&skip| skip.eq_ignore_ascii_case(name.as_ref()))
        } else {
            false
        }
    })
}

/// Hidden file rule: skip if the file name (final component) starts with `.`.
///
/// Allows indexing `.github/workflows/*.yml` while skipping `.env` at workspace root.
pub fn should_skip_hidden_file(rel: &Path) -> bool {
    let Some(stem) = rel.file_name() else {
        return true;
    };
    let s = stem.to_string_lossy();
    s.starts_with('.')
}

/// Byte size gate for a regular file (`metadata.len()`).
pub fn should_skip_by_size(len: u64, min_bytes: u64, max_bytes: u64) -> bool {
    if len < min_bytes {
        return true;
    }
    if len > max_bytes {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_git_and_target_segments() {
        assert!(should_skip_path_components(Path::new(
            "crates/foo/target/debug/x"
        )));
        assert!(should_skip_path_components(Path::new(".git/HEAD")));
        assert!(!should_skip_path_components(Path::new("src/main.rs")));
    }

    #[test]
    fn hidden_file_is_basename_only() {
        assert!(should_skip_hidden_file(Path::new(".env")));
        assert!(!should_skip_hidden_file(Path::new(".github/workflow.yml")));
        assert!(should_skip_hidden_file(Path::new("src/.secrets")));
    }

    #[test]
    fn blocked_extensions_match_case_insensitive() {
        assert!(is_blocked_executable_extension("exe"));
        assert!(is_blocked_executable_extension("DLL"));
        assert!(!is_blocked_executable_extension("rs"));
    }
}
