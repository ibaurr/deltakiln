//! Filesystem helpers: atomic writes and the output-directory safety guard.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Error, Result};

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_sibling(dest: &Path) -> PathBuf {
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    dest.with_file_name(format!(".{name}.tmp-{}-{n}", std::process::id()))
}

fn ensure_parent(dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    Ok(())
}

/// Write `bytes` to `dest` via a temp file in the same directory plus rename,
/// so readers never observe a half-written file.
pub fn write_atomic(dest: &Path, bytes: &[u8]) -> Result<()> {
    ensure_parent(dest)?;
    let tmp = temp_sibling(dest);
    let result = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, dest));
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(Error::io(dest, e));
    }
    Ok(())
}

/// Copy `src` to `dest` atomically (see [`write_atomic`]).
pub fn copy_atomic(src: &Path, dest: &Path) -> Result<()> {
    ensure_parent(dest)?;
    let tmp = temp_sibling(dest);
    let result = std::fs::copy(src, &tmp).and_then(|_| std::fs::rename(&tmp, dest));
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(Error::io(dest, e));
    }
    Ok(())
}

/// Canonicalise `path` even if it does not exist yet, by canonicalising the
/// nearest existing ancestor and re-appending the missing tail.
fn canonicalize_lenient(path: &Path) -> Result<PathBuf> {
    let abs = std::path::absolute(path).map_err(|e| Error::io(path, e))?;
    let mut existing = abs.as_path();
    let mut tail: Vec<&std::ffi::OsStr> = Vec::new();
    loop {
        match existing.canonicalize() {
            Ok(mut canon) => {
                for part in tail.iter().rev() {
                    canon.push(part);
                }
                return Ok(canon);
            }
            Err(_) => match (existing.parent(), existing.file_name()) {
                (Some(parent), Some(name)) => {
                    tail.push(name);
                    existing = parent;
                }
                _ => {
                    return Err(Error::Other(format!(
                        "cannot resolve path {}",
                        path.display()
                    )));
                }
            },
        }
    }
}

/// Refuse to use `output` if it is, or contains, any of `protected`
/// (the site root and its input directories). A build clears its output
/// directory, so `--output .` must never be allowed to wipe the sources.
pub fn ensure_output_is_safe(output: &Path, protected: &[&Path]) -> Result<()> {
    let out = canonicalize_lenient(output)?;
    for p in protected {
        let p = canonicalize_lenient(p)?;
        if p.starts_with(&out) {
            return Err(Error::Other(format!(
                "refusing to use {} as the output directory: it contains {}",
                output.display(),
                p.display()
            )));
        }
    }
    Ok(())
}

/// Remove `dir` (if present) and recreate it empty.
pub fn reset_dir(dir: &Path) -> Result<()> {
    match std::fs::remove_dir_all(dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(Error::io(dir, e)),
    }
    std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("deltakiln-fs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn write_atomic_creates_parents_and_leaves_no_temp_files() {
        let dir = scratch("atomic");
        let dest = dir.join("a/b/c.html");
        write_atomic(&dest, b"one").unwrap();
        write_atomic(&dest, b"two").unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"two");
        let siblings: Vec<_> = std::fs::read_dir(dest.parent().unwrap()).unwrap().collect();
        assert_eq!(siblings.len(), 1);
    }

    #[test]
    fn output_guard_rejects_root_and_ancestors_but_not_children() {
        let root = scratch("guard");
        std::fs::create_dir_all(root.join("content")).unwrap();
        let protected = [root.as_path(), &root.join("content")];
        assert!(ensure_output_is_safe(&root, &protected).is_err());
        assert!(ensure_output_is_safe(&root.join(".."), &protected).is_err());
        assert!(ensure_output_is_safe(&root.join("content"), &protected).is_err());
        assert!(ensure_output_is_safe(&root.join("public"), &protected).is_ok());
        assert!(ensure_output_is_safe(&root.join("deep/not/yet/created"), &protected).is_ok());
    }
}
