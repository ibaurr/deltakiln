//! Markdown to HTML via comrak (GFM extensions).
//!
//! comrak vs pulldown-cmark is still an open question: neither has been
//! benchmarked against the other yet. The choice is isolated behind
//! [`render`] so it can be swapped after the M4 benchmarks.

use comrak::{Options, markdown_to_html};

fn options() -> Options<'static> {
    let mut opts = Options::default();
    // GFM extensions.
    opts.extension.strikethrough = true;
    opts.extension.table = true;
    opts.extension.autolink = true;
    opts.extension.tasklist = true;
    opts.extension.footnotes = true;
    // Anchor ids on headings (empty prefix: `# Hello` gets id="hello").
    opts.extension.header_id_prefix = Some(String::new());
    // Sources are trusted (the site author's own files), and docs routinely
    // embed raw HTML, so pass it through rather than replacing it.
    opts.render.r#unsafe = true;
    opts
}

/// Render a markdown body (front matter already removed) to an HTML fragment.
pub fn render(body: &str) -> String {
    markdown_to_html(body, &options())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_gfm_features() {
        let html = render("| a | b |\n|---|---|\n| 1 | 2 |\n\n~~gone~~\n\n- [x] done\n");
        assert!(html.contains("<table>"), "{html}");
        assert!(html.contains("<del>gone</del>"), "{html}");
        assert!(html.contains("type=\"checkbox\""), "{html}");
    }

    #[test]
    fn headings_get_anchor_ids() {
        assert!(render("# Hello World\n").contains("id=\"hello-world\""));
    }

    #[test]
    fn fenced_code_keeps_language_class() {
        assert!(render("```rust\nfn main() {}\n```\n").contains("language-rust"));
    }

    #[test]
    fn raw_html_passes_through() {
        assert!(render("<div class=\"note\">hi</div>\n").contains("<div class=\"note\">hi</div>"));
    }

    #[test]
    fn rendering_is_deterministic() {
        let src = "# T\n\ntext[^1]\n\n[^1]: note\n";
        assert_eq!(render(src), render(src));
    }
}
