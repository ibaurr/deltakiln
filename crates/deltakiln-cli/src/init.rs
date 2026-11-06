//! `deltakiln init`: scaffold a minimal working site.

use std::path::Path;

use anyhow::{Result, bail};
use deltakiln_core::fsutil::write_atomic;
use deltakiln_render::DEFAULT_PAGE_TEMPLATE;

const CONFIG: &str = "\
# deltakiln.toml - every field is optional.
title = \"My Site\"
# base_url = \"https://example.com\"
";

const INDEX: &str = "\
+++
title = \"Welcome\"
+++
# Welcome

This site was created with `deltakiln init`. Edit the files under `content/`,
then run `deltakiln build`.

Read the [getting started guide](/guide/getting-started/).
";

const GETTING_STARTED: &str = "\
+++
title = \"Getting started\"
description = \"A sample page in a nested directory.\"
+++
# Getting started

`content/guide/getting-started.md` is served at `/guide/getting-started/`.
Files in `static/` are copied to the output as they are.
";

/// Create the starter site under `target`. Refuses to overwrite anything:
/// if any file already exists, nothing is written.
pub fn scaffold(target: &Path) -> Result<Vec<String>> {
    let files: [(&str, &str); 4] = [
        ("deltakiln.toml", CONFIG),
        ("templates/page.html", DEFAULT_PAGE_TEMPLATE),
        ("content/index.md", INDEX),
        ("content/guide/getting-started.md", GETTING_STARTED),
    ];

    let existing: Vec<&str> = files
        .iter()
        .map(|(rel, _)| *rel)
        .filter(|rel| target.join(rel).exists())
        .collect();
    if !existing.is_empty() {
        bail!(
            "refusing to overwrite existing files in {}: {}",
            target.display(),
            existing.join(", ")
        );
    }

    for (rel, contents) in files {
        write_atomic(&target.join(rel), contents.as_bytes())?;
    }
    Ok(files.iter().map(|(rel, _)| rel.to_string()).collect())
}
