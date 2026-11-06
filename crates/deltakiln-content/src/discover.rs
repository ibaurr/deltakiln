use std::path::{Path, PathBuf};

use deltakiln_core::{Error, Result};
use walkdir::WalkDir;

/// A regular file found under a source directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Path relative to the walked directory, always `/`-separated.
    pub rel: String,
    pub abs: PathBuf,
}

/// True for `*.md` (case-insensitive).
pub fn is_markdown(rel: &str) -> bool {
    Path::new(rel)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

/// Recursively list regular files under `dir`, sorted by relative path so the
/// result does not depend on the platform's directory order.
///
/// Symlinks are not followed and not listed. A missing `dir` yields an empty
/// list; callers that require the directory check for it themselves.
/// With `skip_hidden`, names starting with `.` are skipped (files and whole
/// directories); static assets keep them (think `.well-known/`).
pub fn list_files(dir: &Path, skip_hidden: bool) -> Result<Vec<SourceFile>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    let walker = WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !(skip_hidden && e.depth() > 0 && e.file_name().to_string_lossy().starts_with('.'))
        });
    for entry in walker {
        let entry = entry.map_err(|e| {
            let path = e.path().unwrap_or(dir).to_path_buf();
            Error::io(path, e.into())
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel_path = entry
            .path()
            .strip_prefix(dir)
            .expect("walkdir yields paths under its root");
        let mut parts = Vec::new();
        for comp in rel_path.components() {
            let part = comp.as_os_str().to_str().ok_or_else(|| {
                Error::Other(format!(
                    "{}: file name is not valid UTF-8",
                    entry.path().display()
                ))
            })?;
            parts.push(part);
        }
        files.push(SourceFile {
            rel: parts.join("/"),
            abs: entry.path().to_path_buf(),
        });
    }
    files.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_extension_check() {
        assert!(is_markdown("a/b.md"));
        assert!(is_markdown("B.MD"));
        assert!(!is_markdown("a/b.markdown"));
        assert!(!is_markdown("md"));
        assert!(!is_markdown("a/b.md.bak"));
    }

    #[test]
    fn lists_sorted_relative_paths_and_skips_hidden() {
        let dir = std::env::temp_dir().join(format!("deltakiln-discover-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for f in [
            "z.md",
            "a/b.md",
            "a.md",
            ".hidden.md",
            ".git/x.md",
            "a/.secret",
        ] {
            let p = dir.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "x").unwrap();
        }
        let rels = |hidden| {
            list_files(&dir, hidden)
                .unwrap()
                .into_iter()
                .map(|f| f.rel)
                .collect::<Vec<_>>()
        };
        assert_eq!(rels(true), ["a.md", "a/b.md", "z.md"]);
        assert_eq!(
            rels(false),
            [
                ".git/x.md",
                ".hidden.md",
                "a.md",
                "a/.secret",
                "a/b.md",
                "z.md"
            ]
        );
        assert!(list_files(&dir.join("nope"), true).unwrap().is_empty());
    }
}
