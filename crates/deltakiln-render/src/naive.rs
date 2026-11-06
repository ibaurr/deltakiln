//! The naive full build: the permanent correctness oracle.
//!
//! No cache, no dependency graph, no shortcuts: read every source, render
//! every page, copy every static file. It is a pure function of the site
//! directory, and its output is byte-for-byte independent of thread count,
//! run order and operating system. Do not optimise this path; `verify` and the
//! property tests rely on it being obviously correct.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use deltakiln_content::{
    Page, Route, SourceFile, ensure_unique_outputs, is_markdown, list_files, route,
};
use deltakiln_core::fsutil::{copy_atomic, ensure_output_is_safe, reset_dir, write_atomic};
use deltakiln_core::{Config, Error, Result};
use rayon::prelude::*;

use crate::templates::{build_env, render_page, template_name};

#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Site root (the directory holding `deltakiln.toml`, `content/`, ...).
    pub root: PathBuf,
    /// Output directory; defaults to `<root>/<config.output_dir>`.
    pub output: Option<PathBuf>,
    /// Worker threads; `0` means one per core.
    pub jobs: usize,
}

/// One written page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageOutput {
    /// Source path relative to the content directory.
    pub source: String,
    pub url: String,
    /// Output file relative to the output directory.
    pub output_path: String,
}

#[derive(Debug, Clone)]
pub struct BuildReport {
    pub output_dir: PathBuf,
    /// Pages in source-path order.
    pub pages: Vec<PageOutput>,
    pub static_files: usize,
    /// Wall-clock time per phase, in execution order.
    pub timings: Vec<(&'static str, Duration)>,
}

/// Return the values, or the first error in input order. Parallel stages
/// collect every result first so which error is reported never depends on
/// thread scheduling.
fn first_error<T>(results: Vec<Result<T>>) -> Result<Vec<T>> {
    results.into_iter().collect()
}

pub fn naive_build(opts: &BuildOptions) -> Result<BuildReport> {
    let mut timings: Vec<(&'static str, Duration)> = Vec::new();
    let mut phase = |name: &'static str, started: Instant| timings.push((name, started.elapsed()));

    let config = Config::load(&opts.root)?;
    let content_dir = opts.root.join(&config.content_dir);
    let templates_dir = opts.root.join(&config.templates_dir);
    let static_dir = opts.root.join(&config.static_dir);
    let output_dir = opts
        .output
        .clone()
        .unwrap_or_else(|| opts.root.join(&config.output_dir));

    if !content_dir.is_dir() {
        return Err(Error::Other(format!(
            "content directory {} not found (wrong --root? `deltakiln init` creates a starter site)",
            content_dir.display()
        )));
    }

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(opts.jobs)
        .build()
        .map_err(|e| Error::Other(format!("cannot start worker threads: {e}")))?;

    // Discover.
    let started = Instant::now();
    let content_files: Vec<SourceFile> = list_files(&content_dir, true)?
        .into_iter()
        .filter(|f| is_markdown(&f.rel))
        .collect();
    let static_files = list_files(&static_dir, false)?;
    phase("discover", started);

    // Parse front matter (bodies stay as source text; HTML is produced at
    // render time so memory holds sources, not rendered pages).
    let started = Instant::now();
    let parsed = pool.install(|| {
        content_files
            .par_iter()
            .map(|f| {
                let raw = std::fs::read_to_string(&f.abs).map_err(|e| Error::io(&f.abs, e))?;
                Page::parse(f.rel.clone(), &raw)
            })
            .collect::<Vec<_>>()
    });
    let pages = first_error(parsed)?;
    phase("parse", started);

    // Route, then validate everything that can fail before touching the
    // output directory: a broken build must not destroy the previous output.
    let started = Instant::now();
    let routes: Vec<Route> = first_error(pages.iter().map(route).collect())?;
    let env = build_env(&templates_dir)?;
    for page in &pages {
        let name = template_name(page);
        if env.get_template(name).is_err() {
            return Err(Error::content(
                &page.source_path,
                format!(
                    "template `{name}` not found in {}",
                    config.templates_dir.display()
                ),
            ));
        }
    }
    let content_label = config.content_dir.display().to_string();
    let static_label = config.static_dir.display().to_string();
    let page_origins: Vec<String> = pages
        .iter()
        .map(|p| format!("{content_label}/{}", p.source_path))
        .collect();
    let static_origins: Vec<String> = static_files
        .iter()
        .map(|f| format!("{static_label}/{}", f.rel))
        .collect();
    ensure_unique_outputs(
        routes
            .iter()
            .zip(&page_origins)
            .map(|(r, origin)| (r.output_path.as_str(), origin.as_str()))
            .chain(
                static_files
                    .iter()
                    .zip(&static_origins)
                    .map(|(f, origin)| (f.rel.as_str(), origin.as_str())),
            ),
    )?;
    ensure_output_is_safe(
        &output_dir,
        &[&opts.root, &content_dir, &templates_dir, &static_dir],
    )?;
    phase("route", started);

    reset_dir(&output_dir)?;

    // Render and write pages.
    let started = Instant::now();
    let written = pool.install(|| {
        pages
            .par_iter()
            .zip(routes.par_iter())
            .map(|(page, route)| {
                let html = render_page(&env, &config, page, route)?;
                write_atomic(&output_dir.join(&route.output_path), html.as_bytes())
            })
            .collect::<Vec<_>>()
    });
    first_error(written)?;
    phase("render+write", started);

    // Copy static files verbatim.
    let started = Instant::now();
    let copied = pool.install(|| {
        static_files
            .par_iter()
            .map(|f| copy_atomic(&f.abs, &output_dir.join(&f.rel)))
            .collect::<Vec<_>>()
    });
    first_error(copied)?;
    phase("static", started);

    let page_outputs = pages
        .iter()
        .zip(&routes)
        .map(|(page, route)| PageOutput {
            source: page.source_path.clone(),
            url: route.url.clone(),
            output_path: route.output_path.clone(),
        })
        .collect();

    Ok(BuildReport {
        output_dir,
        pages: page_outputs,
        static_files: static_files.len(),
        timings,
    })
}
