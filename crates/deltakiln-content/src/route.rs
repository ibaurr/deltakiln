//! Path-derived routing: `a/b.md` -> `/a/b/` written to `a/b/index.html`.

use std::collections::BTreeMap;

use deltakiln_core::{Error, Result};

use crate::page::Page;

/// Where a page lives on the site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// URL path, percent-encoded, always starting and ending with `/`.
    pub url: String,
    /// Output file relative to the output directory, `/`-separated, unencoded.
    pub output_path: String,
}

/// Reject names that are unsafe or non-portable as a path component.
/// Output must be buildable on Linux, macOS and Windows alike.
fn validate_segment(seg: &str) -> std::result::Result<(), String> {
    if seg.is_empty() {
        return Err("empty path segment".to_string());
    }
    if seg == "." || seg == ".." {
        return Err(format!("path segment `{seg}` is not allowed"));
    }
    if let Some(c) = seg.chars().find(|c| {
        c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    }) {
        return Err(format!(
            "path segment `{seg}` contains unsupported character {c:?}"
        ));
    }
    if seg.ends_with('.') || seg.ends_with(' ') {
        return Err(format!(
            "path segment `{seg}` must not end with `.` or a space"
        ));
    }
    Ok(())
}

fn encode_segment(seg: &str) -> String {
    let mut out = String::with_capacity(seg.len());
    for b in seg.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Compute the route for a page.
///
/// * `index.md` maps to its directory (`/`, `/guide/`).
/// * any other `name.md` maps to `/<dirs>/name/`.
/// * front matter `slug` replaces the last component; `url` replaces everything.
pub fn route(page: &Page) -> Result<Route> {
    let err = |m: String| Error::content(&page.source_path, m);

    let segments: Vec<String> = if let Some(url) = &page.front.url {
        if page.front.slug.is_some() {
            return Err(err("`slug` and `url` cannot both be set".to_string()));
        }
        let path = url
            .strip_prefix('/')
            .ok_or_else(|| err(format!("`url` must start with `/`, got `{url}`")))?;
        if path.is_empty() {
            Vec::new()
        } else {
            let trimmed = path.strip_suffix('/').unwrap_or(path);
            trimmed.split('/').map(str::to_string).collect()
        }
    } else {
        let mut parts: Vec<&str> = page.source_path.split('/').collect();
        let file = parts.pop().unwrap_or_default();
        let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
        let mut segs: Vec<String> = parts.into_iter().map(str::to_string).collect();
        if page.is_index() {
            if page.front.slug.is_some() {
                return Err(err(
                    "`slug` is not allowed on index pages; use `url` instead".to_string(),
                ));
            }
        } else {
            segs.push(page.front.slug.clone().unwrap_or_else(|| stem.to_string()));
        }
        segs
    };

    for seg in &segments {
        validate_segment(seg).map_err(err)?;
    }

    if segments.is_empty() {
        return Ok(Route {
            url: "/".to_string(),
            output_path: "index.html".to_string(),
        });
    }
    let encoded: Vec<String> = segments.iter().map(|s| encode_segment(s)).collect();
    Ok(Route {
        url: format!("/{}/", encoded.join("/")),
        output_path: format!("{}/index.html", segments.join("/")),
    })
}

/// Check that no two outputs collide. `entries` are `(output_path, origin)`
/// pairs where `origin` is a human-readable label for error messages.
///
/// Comparison is case-insensitive because the output must be buildable on
/// case-insensitive filesystems (Windows, default macOS), and a file may not
/// also be needed as a directory (`foo` vs `foo/index.html`).
pub fn ensure_unique_outputs<'a>(
    entries: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<()> {
    let entries: Vec<(&str, &str)> = entries.into_iter().collect();

    let mut files: BTreeMap<String, (&str, &str)> = BTreeMap::new();
    for &(output, origin) in &entries {
        if let Some(&(_, first)) = files.get(&output.to_lowercase()) {
            return Err(Error::Collision {
                output: output.to_string(),
                first: first.to_string(),
                second: origin.to_string(),
            });
        }
        files.insert(output.to_lowercase(), (output, origin));
    }

    let mut dirs: BTreeMap<String, &str> = BTreeMap::new();
    for &(output, origin) in &entries {
        let lower = output.to_lowercase();
        let mut end = 0;
        while let Some(i) = lower[end..].find('/') {
            end += i;
            dirs.entry(lower[..end].to_string()).or_insert(origin);
            end += 1;
        }
    }
    for (lower, &(output, origin)) in &files {
        if let Some(dir_origin) = dirs.get(lower) {
            return Err(Error::Other(format!(
                "output conflict: {origin} writes the file `{output}`, but {dir_origin} needs `{output}` to be a directory"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(path: &str, front: &str) -> Page {
        let src = if front.is_empty() {
            String::new()
        } else {
            format!("+++\n{front}\n+++\n")
        };
        Page::parse(path, &src).unwrap()
    }

    fn r(path: &str, front: &str) -> Result<Route> {
        route(&page(path, front))
    }

    #[test]
    fn clean_urls() {
        let cases = [
            ("index.md", "/", "index.html"),
            ("about.md", "/about/", "about/index.html"),
            ("a/b.md", "/a/b/", "a/b/index.html"),
            ("a/index.md", "/a/", "a/index.html"),
            ("a/b/index.md", "/a/b/", "a/b/index.html"),
        ];
        for (src, url, out) in cases {
            let route = r(src, "").unwrap();
            assert_eq!(
                (route.url.as_str(), route.output_path.as_str()),
                (url, out),
                "{src}"
            );
        }
    }

    #[test]
    fn slug_replaces_last_component() {
        let route = r("a/b.md", "slug = \"renamed\"").unwrap();
        assert_eq!(route.url, "/a/renamed/");
        assert_eq!(route.output_path, "a/renamed/index.html");
    }

    #[test]
    fn url_replaces_everything() {
        assert_eq!(
            r("a/b.md", "url = \"/x/y/\"").unwrap().output_path,
            "x/y/index.html"
        );
        assert_eq!(r("a/b.md", "url = \"/x/y\"").unwrap().url, "/x/y/");
        assert_eq!(
            r("a/b.md", "url = \"/\"").unwrap().output_path,
            "index.html"
        );
    }

    #[test]
    fn bad_overrides_are_errors() {
        for front in [
            "url = \"x/y\"",
            "url = \"//\"",
            "url = \"/a//b\"",
            "url = \"/a/../b\"",
            "slug = \"a/b\"",
            "slug = \"\"",
            "slug = \"..\"",
            "slug = \"a\"\nurl = \"/b/\"",
            "slug = \"bad:name\"",
        ] {
            assert!(r("p.md", front).is_err(), "{front}");
        }
        assert!(r("index.md", "slug = \"x\"").is_err());
    }

    #[test]
    fn unportable_source_names_are_errors() {
        assert!(r("a:b.md", "").is_err());
        assert!(r("trailing./x.md", "").is_err());
    }

    #[test]
    fn spaces_and_unicode_are_encoded_in_url_but_not_in_output_path() {
        let route = r("My Page/é.md", "").unwrap();
        assert_eq!(route.url, "/My%20Page/%C3%A9/");
        assert_eq!(route.output_path, "My Page/é/index.html");
    }

    #[test]
    fn collisions_between_pages() {
        let a = r("a.md", "").unwrap();
        let b = r("b.md", "url = \"/a/\"").unwrap();
        let err = ensure_unique_outputs([
            (a.output_path.as_str(), "content/a.md"),
            (b.output_path.as_str(), "content/b.md"),
        ])
        .unwrap_err();
        assert!(matches!(err, Error::Collision { .. }), "{err}");
        assert!(
            err.to_string().contains("content/a.md") && err.to_string().contains("content/b.md")
        );
    }

    #[test]
    fn collisions_are_case_insensitive() {
        assert!(ensure_unique_outputs([("A/index.html", "x"), ("a/index.html", "y")]).is_err());
    }

    #[test]
    fn file_versus_directory_conflict() {
        let err =
            ensure_unique_outputs([("foo", "static/foo"), ("foo/index.html", "content/foo.md")])
                .unwrap_err();
        assert!(err.to_string().contains("directory"), "{err}");
    }

    #[test]
    fn distinct_outputs_are_fine() {
        assert!(
            ensure_unique_outputs([
                ("index.html", "a"),
                ("a/index.html", "b"),
                ("style.css", "c"),
                ("img/logo.png", "d"),
            ])
            .is_ok()
        );
    }
}
