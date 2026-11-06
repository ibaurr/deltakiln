use std::borrow::Cow;

use deltakiln_core::{Error, Result};
use serde::Deserialize;

/// Parsed `+++`-delimited TOML front matter. YAML front matter is not
/// supported yet (planned for M1 if needed).
///
/// Unknown keys are an error rather than silently ignored (a typo like
/// `titel` should not vanish); free-form data belongs under `[extra]`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub description: Option<String>,
    pub date: Option<toml::value::Datetime>,
    /// Replaces the last path component of the page's URL.
    pub slug: Option<String>,
    /// Replaces the whole URL path (must start with `/`).
    pub url: Option<String>,
    /// Template name; defaults to `page.html`.
    pub template: Option<String>,
    /// Free-form data exposed to templates as `page.extra`.
    pub extra: toml::Table,
}

/// A markdown source file split into front matter and body.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// Path relative to the content directory, `/`-separated (e.g. `guide/intro.md`).
    pub source_path: String,
    pub front: FrontMatter,
    /// Markdown body with the front matter removed.
    pub body: String,
}

/// Strip a UTF-8 BOM and convert CRLF to LF so a Windows checkout produces
/// exactly the bytes a Linux checkout does.
pub fn normalize_source(raw: &str) -> Cow<'_, str> {
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    if raw.contains('\r') {
        Cow::Owned(raw.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(raw)
    }
}

/// Split `source` (already normalised) into `(front matter text, body)`.
fn split_front_matter(source: &str) -> std::result::Result<(Option<&str>, &str), String> {
    if let Some(rest) = source.strip_prefix("+++\n") {
        let mut offset = 0;
        for line in rest.split_inclusive('\n') {
            if line.trim_end_matches('\n') == "+++" {
                return Ok((Some(&rest[..offset]), &rest[offset + line.len()..]));
            }
            offset += line.len();
        }
        return Err("front matter opened with `+++` but is never closed".to_string());
    }
    if let Some(rest) = source.strip_prefix("---\n")
        && rest.split('\n').any(|l| l == "---")
    {
        return Err(
            "YAML front matter (`---`) is not supported yet; use TOML between `+++` lines"
                .to_string(),
        );
    }
    Ok((None, source))
}

impl Page {
    /// Parse a markdown file. `raw` is the file as read from disk.
    pub fn parse(source_path: impl Into<String>, raw: &str) -> Result<Page> {
        let source_path = source_path.into();
        let source = normalize_source(raw);
        let (fm_text, body) =
            split_front_matter(&source).map_err(|m| Error::content(&source_path, m))?;
        let front = match fm_text {
            Some(text) => toml::from_str(text)
                .map_err(|e| Error::content(&source_path, format!("invalid front matter: {e}")))?,
            None => FrontMatter::default(),
        };
        Ok(Page {
            source_path,
            front,
            body: body.to_string(),
        })
    }

    /// `index.md` (at any depth) is the page for its directory.
    pub fn is_index(&self) -> bool {
        self.file_stem() == "index"
    }

    fn file_stem(&self) -> &str {
        let name = self.source_path.rsplit('/').next().unwrap_or("");
        name.rsplit_once('.').map_or(name, |(stem, _)| stem)
    }

    /// Front matter title, else a name derived from the path.
    pub fn title(&self, site_title: &str) -> String {
        if let Some(t) = &self.front.title {
            return t.clone();
        }
        if self.is_index() {
            let mut parts = self.source_path.rsplit('/');
            parts.next();
            return parts
                .next()
                .map_or_else(|| site_title.to_string(), str::to_string);
        }
        self.file_stem().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_without_front_matter() {
        let p = Page::parse("a.md", "# Hi\n").unwrap();
        assert_eq!(p.front, FrontMatter::default());
        assert_eq!(p.body, "# Hi\n");
    }

    #[test]
    fn toml_front_matter_is_split_and_parsed() {
        let src = "+++\ntitle = \"T\"\ndate = 2024-05-01\n[extra]\nk = 1\n+++\nbody\n";
        let p = Page::parse("a.md", src).unwrap();
        assert_eq!(p.front.title.as_deref(), Some("T"));
        assert_eq!(p.front.date.unwrap().to_string(), "2024-05-01");
        assert_eq!(p.front.extra["k"].as_integer(), Some(1));
        assert_eq!(p.body, "body\n");
    }

    #[test]
    fn empty_front_matter_and_empty_body() {
        let p = Page::parse("a.md", "+++\n+++\n").unwrap();
        assert_eq!(p.front, FrontMatter::default());
        assert_eq!(p.body, "");
    }

    #[test]
    fn crlf_and_bom_are_normalised() {
        let p = Page::parse("a.md", "\u{feff}+++\r\ntitle = \"T\"\r\n+++\r\nx\r\ny\r\n").unwrap();
        assert_eq!(p.front.title.as_deref(), Some("T"));
        assert_eq!(p.body, "x\ny\n");
    }

    #[test]
    fn plus_signs_inside_body_are_not_delimiters() {
        let p = Page::parse("a.md", "+++\ntitle = \"T\"\n+++\ntext\n+++\nmore\n").unwrap();
        assert_eq!(p.body, "text\n+++\nmore\n");
    }

    #[test]
    fn unterminated_front_matter_is_an_error_naming_the_file() {
        let err = Page::parse("docs/a.md", "+++\ntitle = \"T\"\n").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.starts_with("docs/a.md:") && msg.contains("never closed"),
            "{msg}"
        );
    }

    #[test]
    fn invalid_toml_is_an_error() {
        let err = Page::parse("a.md", "+++\ntitle = \n+++\n").unwrap_err();
        assert!(err.to_string().contains("invalid front matter"));
    }

    #[test]
    fn unknown_top_level_keys_are_rejected() {
        let err = Page::parse("a.md", "+++\ntitel = \"T\"\n+++\n").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("titel") && msg.starts_with("a.md:"), "{msg}");
    }

    #[test]
    fn yaml_front_matter_gets_a_clear_error() {
        let err = Page::parse("a.md", "---\ntitle: T\n---\nbody\n").unwrap_err();
        assert!(err.to_string().contains("YAML front matter"));
    }

    #[test]
    fn thematic_break_at_top_is_still_markdown() {
        let p = Page::parse("a.md", "---\njust a rule above\n").unwrap();
        assert_eq!(p.body, "---\njust a rule above\n");
    }

    #[test]
    fn titles() {
        let t = |path: &str| Page::parse(path, "").unwrap().title("Site");
        assert_eq!(t("guide/intro.md"), "intro");
        assert_eq!(t("index.md"), "Site");
        assert_eq!(t("guide/index.md"), "guide");
        let p = Page::parse("x.md", "+++\ntitle = \"Given\"\n+++\n").unwrap();
        assert_eq!(p.title("Site"), "Given");
    }
}
