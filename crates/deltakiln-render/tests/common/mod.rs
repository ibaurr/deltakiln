#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use deltakiln_content::list_files;
use deltakiln_render::{BuildOptions, BuildReport, naive_build};

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Every file under `dir` (hidden ones included) as relative path -> bytes.
pub fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    list_files(dir, false)
        .unwrap()
        .into_iter()
        .map(|f| (f.rel, std::fs::read(&f.abs).unwrap()))
        .collect()
}

pub fn assert_same(actual: &BTreeMap<String, Vec<u8>>, expected: &BTreeMap<String, Vec<u8>>) {
    let a: Vec<_> = actual.keys().collect();
    let e: Vec<_> = expected.keys().collect();
    assert_eq!(a, e, "different set of output files");
    for (path, bytes) in expected {
        assert!(
            &actual[path] == bytes,
            "{path} differs:\n--- expected\n{}\n--- actual\n{}",
            String::from_utf8_lossy(bytes),
            String::from_utf8_lossy(&actual[path]),
        );
    }
}

pub fn build(root: &Path, out: &Path, jobs: usize) -> BuildReport {
    try_build(root, out, jobs).unwrap_or_else(|e| panic!("build failed: {e}"))
}

pub fn try_build(root: &Path, out: &Path, jobs: usize) -> deltakiln_core::Result<BuildReport> {
    naive_build(&BuildOptions {
        root: root.to_path_buf(),
        output: Some(out.to_path_buf()),
        jobs,
    })
}

pub fn copy_dir(src: &Path, dst: &Path) {
    for f in list_files(src, false).unwrap() {
        let dest = dst.join(&f.rel);
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::copy(&f.abs, dest).unwrap();
    }
}

/// Create a site in a fresh temp dir from `(relative path, contents)` pairs.
pub fn site(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, contents) in files {
        let p = dir.path().join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, contents).unwrap();
    }
    dir
}
