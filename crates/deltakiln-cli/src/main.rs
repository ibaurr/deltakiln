mod init;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use deltakiln_render::{BuildOptions, naive_build};

/// Incremental-first static site generator for very large markdown doc sets.
#[derive(Parser)]
#[command(name = "deltakiln", version, about)]
struct Cli {
    /// Site root (directory holding deltakiln.toml, content/, ...).
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,

    /// Output directory (default: `public` under the root, or `output_dir` from config).
    #[arg(long, global = true)]
    output: Option<PathBuf>,

    /// Worker threads (0 = one per core).
    #[arg(long, global = true, default_value_t = 0)]
    jobs: usize,

    /// Print every page written.
    #[arg(long, global = true)]
    verbose: bool,

    /// Print per-phase timings.
    #[arg(long, global = true)]
    timings: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold a minimal site (config, one layout, sample content).
    Init {
        /// Directory to create the site in (default: --root).
        path: Option<PathBuf>,
    },
    /// Build the site into the output directory.
    Build {
        /// Ignore the cache. Until the cache exists (M1) every build is a clean full build.
        #[arg(long)]
        clean: bool,
    },
    /// Dev server with file watching and live reload (M3).
    Serve,
    /// Full rebuild diffed against the incremental output (M1).
    Verify,
    /// Explain why a page was or was not rebuilt (M3).
    Explain {
        /// Source path of the page.
        path: PathBuf,
    },
    /// Run the built-in benchmark (M4).
    Bench,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match &cli.command {
        Command::Init { path } => {
            let target = path.as_ref().unwrap_or(&cli.root);
            let created = init::scaffold(target)?;
            println!("Created a new site in {}:", target.display());
            for file in created {
                println!("  {file}");
            }
            println!("Next: deltakiln build --root {}", target.display());
            Ok(())
        }
        Command::Build { clean: _ } => build(&cli),
        Command::Serve => not_yet("serve", "M3"),
        Command::Verify => not_yet("verify", "M1"),
        Command::Explain { .. } => not_yet("explain", "M3"),
        Command::Bench => not_yet("bench", "M4"),
    }
}

fn not_yet(command: &str, milestone: &str) -> Result<()> {
    bail!("`{command}` is not implemented yet (planned for {milestone})")
}

fn build(cli: &Cli) -> Result<()> {
    let started = Instant::now();
    let report = naive_build(&BuildOptions {
        root: cli.root.clone(),
        output: cli.output.clone(),
        jobs: cli.jobs,
    })
    .with_context(|| format!("build failed in {}", cli.root.display()))?;

    if cli.verbose {
        for page in &report.pages {
            println!("  {} -> {}", page.source, page.output_path);
        }
    }
    println!(
        "Built {} pages and {} static files into {} in {} ms",
        report.pages.len(),
        report.static_files,
        report.output_dir.display(),
        started.elapsed().as_millis()
    );
    if cli.timings {
        for (phase, duration) in &report.timings {
            println!("  {phase:<14} {:>8.2} ms", duration.as_secs_f64() * 1000.0);
        }
    }
    Ok(())
}
