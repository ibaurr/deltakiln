use std::path::Path;
use std::process::{Command, Output};

fn deltakiln(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deltakiln"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run deltakiln")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn init_then_build_produces_a_working_site() {
    let dir = tempfile::tempdir().unwrap();
    let init = deltakiln(&["init", "demo"], dir.path());
    assert!(init.status.success(), "{}", stderr(&init));

    let site = dir.path().join("demo");
    for f in [
        "deltakiln.toml",
        "templates/page.html",
        "content/index.md",
        "content/guide/getting-started.md",
    ] {
        assert!(site.join(f).is_file(), "{f} missing");
    }

    let build = deltakiln(&["build", "--root", "demo"], dir.path());
    assert!(build.status.success(), "{}", stderr(&build));
    assert!(
        stdout(&build).contains("Built 2 pages"),
        "{}",
        stdout(&build)
    );

    let home = std::fs::read_to_string(site.join("public/index.html")).unwrap();
    assert!(
        home.contains("<title>My Site</title>") && home.contains("<h1 id=\"welcome\">"),
        "{home}"
    );
    assert!(
        site.join("public/guide/getting-started/index.html")
            .is_file()
    );
}

#[test]
fn init_uses_root_when_no_path_is_given() {
    let dir = tempfile::tempdir().unwrap();
    let out = deltakiln(&["init"], dir.path());
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(dir.path().join("content/index.md").is_file());
}

#[test]
fn init_refuses_to_overwrite_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("content")).unwrap();
    std::fs::write(dir.path().join("content/index.md"), "mine\n").unwrap();

    let out = deltakiln(&["init"], dir.path());
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("content/index.md"),
        "{}",
        stderr(&out)
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("content/index.md")).unwrap(),
        "mine\n"
    );
    assert!(
        !dir.path().join("deltakiln.toml").exists(),
        "partial scaffold left behind"
    );
}

#[test]
fn build_flags_output_jobs_verbose_timings() {
    let dir = tempfile::tempdir().unwrap();
    assert!(deltakiln(&["init"], dir.path()).status.success());
    let out = deltakiln(
        &[
            "--output",
            "dist",
            "--jobs",
            "2",
            "--verbose",
            "--timings",
            "build",
            "--clean",
        ],
        dir.path(),
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("index.md -> index.html"), "{text}");
    assert!(text.contains("render+write"), "{text}");
    assert!(dir.path().join("dist/index.html").is_file());
    assert!(!dir.path().join("public").exists());
}

#[test]
fn build_error_exits_nonzero_with_message() {
    let dir = tempfile::tempdir().unwrap();
    let out = deltakiln(&["build"], dir.path());
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("content directory"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn later_milestone_commands_fail_clearly() {
    let dir = tempfile::tempdir().unwrap();
    for (args, milestone) in [
        (vec!["serve"], "M3"),
        (vec!["verify"], "M1"),
        (vec!["explain", "a.md"], "M3"),
        (vec!["bench"], "M4"),
    ] {
        let out = deltakiln(&args, dir.path());
        assert!(!out.status.success(), "{args:?}");
        let err = stderr(&out);
        assert!(
            err.contains("not implemented yet") && err.contains(milestone),
            "{args:?}: {err}"
        );
    }
}
