//! The oracle must be a pure function of the site directory.

mod common;

use common::*;

#[test]
fn output_is_identical_across_thread_counts_and_runs() {
    let root = fixture("basic");
    let reference = tempfile::tempdir().unwrap();
    build(&root, reference.path(), 1);
    let reference = snapshot(reference.path());

    for jobs in [1, 2, 8, 0] {
        for _ in 0..3 {
            let out = tempfile::tempdir().unwrap();
            build(&root, out.path(), jobs);
            assert_same(&snapshot(out.path()), &reference);
        }
    }
}

#[test]
fn rebuilding_into_the_same_directory_is_stable_and_removes_stale_files() {
    let root = fixture("basic");
    let out = tempfile::tempdir().unwrap();
    build(&root, out.path(), 0);
    let first = snapshot(out.path());

    std::fs::write(out.path().join("stale.html"), "left over").unwrap();
    std::fs::create_dir_all(out.path().join("old/dir")).unwrap();
    std::fs::write(out.path().join("old/dir/page.html"), "left over").unwrap();

    build(&root, out.path(), 0);
    assert_same(&snapshot(out.path()), &first);
}

#[test]
fn crlf_checkout_builds_the_same_bytes_as_lf() {
    let lf = tempfile::tempdir().unwrap();
    let crlf = tempfile::tempdir().unwrap();
    copy_dir(&fixture("basic"), lf.path());
    copy_dir(&fixture("basic"), crlf.path());

    // Rewrite every text source (not static assets, which are copied verbatim)
    // with CRLF line endings plus a BOM on one file.
    for dir in ["content", "templates"] {
        for f in deltakiln_content::list_files(&crlf.path().join(dir), false).unwrap() {
            let text = std::fs::read_to_string(&f.abs).unwrap();
            std::fs::write(&f.abs, text.replace('\n', "\r\n")).unwrap();
        }
    }
    let toml = crlf.path().join("deltakiln.toml");
    let text = std::fs::read_to_string(&toml).unwrap();
    std::fs::write(&toml, text.replace('\n', "\r\n")).unwrap();
    let about = crlf.path().join("content/about.md");
    let mut bytes = b"\xEF\xBB\xBF".to_vec();
    bytes.extend(std::fs::read(&about).unwrap());
    std::fs::write(&about, bytes).unwrap();

    let out_lf = tempfile::tempdir().unwrap();
    let out_crlf = tempfile::tempdir().unwrap();
    build(lf.path(), out_lf.path(), 0);
    build(crlf.path(), out_crlf.path(), 0);
    assert_same(&snapshot(out_crlf.path()), &snapshot(out_lf.path()));
}
