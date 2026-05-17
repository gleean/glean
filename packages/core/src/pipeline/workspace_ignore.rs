//! Workspace-relative `.gitignore` then `.gleanignore` (merge order per `file-system-rules.md`).

use std::path::Path;

use ignore::gitignore::GitignoreBuilder;

/// Compiled ignore patterns rooted at `workspace_root` (ripgrep-style semantics via `ignore` crate).
pub struct WorkspaceIgnore {
    inner: ignore::gitignore::Gitignore,
}

impl WorkspaceIgnore {
    /// Loads ignore files under `workspace_root`. When `use_gitignore` is false, skips `.gitignore` but still loads `.gleanignore`.
    pub fn load(workspace_root: &Path, use_gitignore: bool) -> Result<Self, ignore::Error> {
        let mut builder = GitignoreBuilder::new(workspace_root);
        if use_gitignore {
            let gitignore = workspace_root.join(".gitignore");
            if gitignore.is_file() {
                builder.add(&gitignore);
            }
        }
        let gleanignore = workspace_root.join(".gleanignore");
        if gleanignore.is_file() {
            builder.add(&gleanignore);
        }
        let inner = builder.build()?;
        Ok(Self { inner })
    }

    /// `rel` must be workspace-relative (same convention as [`WalkDir`] / shadow `path_key`).
    pub fn is_ignored(&self, rel: &Path, is_dir: bool) -> bool {
        if self.inner.matched(rel, is_dir).is_ignore() {
            return true;
        }
        // Directory patterns like `skipme/` ignore descendants; walk parents for matches.
        let mut p = rel;
        while let Some(parent) = p.parent() {
            if parent.as_os_str().is_empty() {
                break;
            }
            if self.inner.matched(parent, true).is_ignore() {
                return true;
            }
            p = parent;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn gleanignore_can_exclude_dotfile() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join(".gleanignore"), "*.env\n").unwrap();
        let ig = WorkspaceIgnore::load(root, true).unwrap();
        assert!(ig.is_ignored(Path::new("secrets.env"), false));
        assert!(!ig.is_ignored(Path::new(".github/ci.yml"), false));
    }

    #[test]
    fn dot_github_md_still_visible_without_ignore() {
        let dir = tempdir().unwrap();
        let ig = WorkspaceIgnore::load(dir.path(), true).unwrap();
        assert!(!ig.is_ignored(Path::new(".github/workflows/x.md"), false));
    }
}
