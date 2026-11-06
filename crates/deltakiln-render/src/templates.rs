//! minijinja environment and the template context.
//!
//! Design rule (PRD 5.2): a template may read only the page's own data and the
//! site config. There is deliberately no way to query other pages from here;
//! cross-page data will arrive as explicit derived nodes (M2).

use std::path::Path;

use deltakiln_content::page::normalize_source;
use deltakiln_content::{Page, Route, list_files};
use deltakiln_core::{Config, Error, Result};
use minijinja::value::Value;
use minijinja::{Environment, UndefinedBehavior, context};

/// Layout used when the site does not provide `templates/page.html`.
pub const DEFAULT_PAGE_TEMPLATE: &str = include_str!("default_page.html");

/// Template used for pages that do not set `template` in front matter.
pub const PAGE_TEMPLATE: &str = "page.html";

/// Load every file under `templates_dir` (named by its relative path) plus the
/// built-in default `page.html` if the site has none.
pub fn build_env(templates_dir: &Path) -> Result<Environment<'static>> {
    let mut env = Environment::new();
    // Printing or iterating an undefined value is an error instead of a silent
    // empty string, so typos in templates cannot produce quietly wrong pages.
    env.set_undefined_behavior(UndefinedBehavior::SemiStrict);

    let mut has_page_template = false;
    for file in list_files(templates_dir, true)? {
        let raw = std::fs::read_to_string(&file.abs).map_err(|e| Error::io(&file.abs, e))?;
        let text = normalize_source(&raw).into_owned();
        has_page_template |= file.rel == PAGE_TEMPLATE;
        env.add_template_owned(file.rel.clone(), text)
            .map_err(|e| Error::Template(format!("{}: {e:#}", file.rel)))?;
    }
    if !has_page_template {
        env.add_template_owned(PAGE_TEMPLATE, DEFAULT_PAGE_TEMPLATE)
            .map_err(|e| Error::Template(format!("built-in {PAGE_TEMPLATE}: {e:#}")))?;
    }
    Ok(env)
}

/// Name of the template a page renders with.
pub fn template_name(page: &Page) -> &str {
    page.front.template.as_deref().unwrap_or(PAGE_TEMPLATE)
}

fn toml_to_value(v: &toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::from(s.as_str()),
        toml::Value::Integer(i) => Value::from(*i),
        toml::Value::Float(f) => Value::from(*f),
        toml::Value::Boolean(b) => Value::from(*b),
        toml::Value::Datetime(d) => Value::from(d.to_string()),
        toml::Value::Array(a) => Value::from(a.iter().map(toml_to_value).collect::<Vec<_>>()),
        toml::Value::Table(t) => {
            Value::from_pairs(t.iter().map(|(k, v)| (k.as_str(), toml_to_value(v))))
        }
    }
}

/// Render one page: markdown body to HTML, then through its template.
pub fn render_page(
    env: &Environment<'_>,
    config: &Config,
    page: &Page,
    route: &Route,
) -> Result<String> {
    let template_err =
        |e: minijinja::Error| Error::Template(format!("{}: {e:#}", page.source_path));

    let content = deltakiln_content::markdown::render(&page.body);
    let ctx = context! {
        site => context! {
            title => config.title.as_str(),
            base_url => config.base_url.as_str(),
        },
        page => context! {
            title => page.title(&config.title),
            description => page.front.description.clone(),
            date => page.front.date.as_ref().map(ToString::to_string),
            url => route.url.as_str(),
            source_path => page.source_path.as_str(),
            content => Value::from_safe_string(content),
            extra => toml_to_value(&toml::Value::Table(page.front.extra.clone())),
        },
    };
    env.get_template(template_name(page))
        .and_then(|t| t.render(ctx))
        .map_err(template_err)
}
