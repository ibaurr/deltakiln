//! Error paths: clear messages, and a failed build never wrecks good output.

mod common;

use common::*;

const MINIMAL: &[(&str, &str)] = &[("content/index.md", "# Hi\n")];

fn err_of(files: &[(&str, &str)]) -> String {
    let site = site(files);
    let out = tempfile::tempdir().unwrap();
    try_build(site.path(), out.path(), 0)
        .expect_err("build should fail")
        .to_string()
}

#[test]
fn zero_config_site_builds_with_builtin_layout() {
    let site = site(&[
        ("content/index.md", "# Hello\n\nworld\n"),
        ("content/docs/page.md", "text\n"),
    ]);
    let out = tempfile::tempdir().unwrap();
    let report = build(site.path(), out.path(), 0);
    assert_eq!(report.pages.len(), 2);
    let html = std::fs::read_to_string(out.path().join("index.html")).unwrap();
    assert!(html.contains("<h1 id=\"hello\">"), "{html}");
    assert!(html.contains("<title>My Site</title>"), "{html}");
    assert!(out.path().join("docs/page/index.html").is_file());
}

#[test]
fn relative_links_pass_through_unchanged() {
    let site = site(&[("content/a.md", "[x](../other/) and [y](sibling.md)\n")]);
    let out = tempfile::tempdir().unwrap();
    build(site.path(), out.path(), 0);
    let html = std::fs::read_to_string(out.path().join("a/index.html")).unwrap();
    assert!(
        html.contains("href=\"../other/\"") && html.contains("href=\"sibling.md\""),
        "{html}"
    );
}

#[test]
fn missing_content_dir_is_a_clear_error() {
    let msg = err_of(&[("deltakiln.toml", "title = \"x\"\n")]);
    assert!(
        msg.contains("content directory") && msg.contains("not found"),
        "{msg}"
    );
}

#[test]
fn bad_front_matter_names_the_file() {
    let msg = err_of(&[
        ("content/ok.md", "fine\n"),
        ("content/docs/bad.md", "+++\ntitle = \n+++\n"),
    ]);
    assert!(msg.starts_with("docs/bad.md:"), "{msg}");
}

#[test]
fn yaml_front_matter_is_rejected_with_guidance() {
    let msg = err_of(&[("content/y.md", "---\ntitle: x\n---\nbody\n")]);
    assert!(msg.contains("YAML") && msg.contains("+++"), "{msg}");
}

#[test]
fn first_error_in_source_order_is_reported_deterministically() {
    let files: Vec<(String, &str)> = (0..40)
        .map(|i| (format!("content/p{i:02}.md"), "+++\nbroken = \n+++\n"))
        .collect();
    let refs: Vec<(&str, &str)> = files.iter().map(|(p, c)| (p.as_str(), *c)).collect();
    for _ in 0..5 {
        assert!(err_of(&refs).starts_with("p00.md:"));
    }
}

#[test]
fn colliding_pages_are_reported_with_both_sources() {
    let msg = err_of(&[
        ("content/a.md", "x\n"),
        ("content/b.md", "+++\nurl = \"/a/\"\n+++\n"),
    ]);
    assert!(
        msg.contains("content/a.md") && msg.contains("content/b.md"),
        "{msg}"
    );
}

#[test]
fn page_versus_static_collision_is_detected() {
    let msg = err_of(&[
        ("content/a.md", "x\n"),
        ("static/a/index.html", "static wins?\n"),
    ]);
    assert!(
        msg.contains("content/a.md") && msg.contains("static/a/index.html"),
        "{msg}"
    );
}

#[test]
fn unknown_template_is_an_error() {
    let msg = err_of(&[("content/a.md", "+++\ntemplate = \"nope.html\"\n+++\n")]);
    assert!(msg.contains("a.md") && msg.contains("nope.html"), "{msg}");
}

#[test]
fn undefined_template_variable_is_an_error_not_an_empty_string() {
    let msg = err_of(&[
        ("content/a.md", "x\n"),
        ("templates/page.html", "{{ page.does_not_exist }}"),
    ]);
    assert!(msg.contains("a.md"), "{msg}");
}

#[test]
fn template_syntax_error_names_the_template() {
    let msg = err_of(&[("content/a.md", "x\n"), ("templates/page.html", "{% if %}")]);
    assert!(msg.contains("page.html"), "{msg}");
}

#[test]
fn failed_build_leaves_previous_output_untouched() {
    let site = site(MINIMAL);
    let out = tempfile::tempdir().unwrap();
    build(site.path(), out.path(), 0);
    let before = snapshot(out.path());

    std::fs::write(site.path().join("content/bad.md"), "+++\nx = \n+++\n").unwrap();
    assert!(try_build(site.path(), out.path(), 0).is_err());
    assert_same(&snapshot(out.path()), &before);
}

#[test]
fn output_dir_that_contains_the_sources_is_refused() {
    let site = site(MINIMAL);
    for out in [
        site.path().to_path_buf(),
        site.path().join("content"),
        site.path().join(".."),
    ] {
        let err = try_build(site.path(), &out, 0)
            .expect_err("must refuse")
            .to_string();
        assert!(err.contains("refusing"), "{err}");
    }
    assert!(
        site.path().join("content/index.md").is_file(),
        "sources must survive"
    );
}
