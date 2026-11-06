//! Golden test: the fixture site must build to exactly `basic.expected/`.
//!
//! Regenerate after an intentional output change with
//! `UPDATE_GOLDEN=1 cargo test -p deltakiln-render --test golden`
//! and review the diff before committing.

mod common;

use common::*;

#[test]
fn basic_site_matches_golden_output() {
    let out = tempfile::tempdir().unwrap();
    let report = build(&fixture("basic"), out.path(), 0);
    assert_eq!(report.pages.len(), 7);
    assert_eq!(report.static_files, 3);

    let expected_dir = fixture("basic.expected");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        let _ = std::fs::remove_dir_all(&expected_dir);
        std::fs::create_dir_all(&expected_dir).unwrap();
        copy_dir(out.path(), &expected_dir);
        return;
    }
    assert_same(&snapshot(out.path()), &snapshot(&expected_dir));
}

#[test]
fn routes_follow_the_clean_url_convention() {
    let out = tempfile::tempdir().unwrap();
    let report = build(&fixture("basic"), out.path(), 0);
    let routes: Vec<(&str, &str)> = report
        .pages
        .iter()
        .map(|p| (p.source.as_str(), p.output_path.as_str()))
        .collect();
    assert_eq!(
        routes,
        [
            ("about.md", "about/index.html"),
            (
                "guide/advanced/deep-dive.md",
                "guide/advanced/internals/index.html"
            ),
            (
                "guide/getting-started.md",
                "guide/getting-started/index.html"
            ),
            ("guide/index.md", "guide/index.html"),
            ("index.md", "index.html"),
            ("notes/My Note.md", "notes/My Note/index.html"),
            ("special.md", "custom/path/index.html"),
        ]
    );
}

#[test]
fn static_files_are_copied_verbatim_preserving_structure() {
    let out = tempfile::tempdir().unwrap();
    build(&fixture("basic"), out.path(), 0);
    for rel in ["style.css", ".well-known/security.txt", "img/blob.bin"] {
        let original = std::fs::read(fixture("basic").join("static").join(rel)).unwrap();
        assert_eq!(
            std::fs::read(out.path().join(rel)).unwrap(),
            original,
            "{rel}"
        );
    }
}
