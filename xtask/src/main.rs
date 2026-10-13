//! Workspace automation. Run with `cargo xtask <task>`.

mod assets;
mod layers;
mod perf;
mod scorecard;
mod stats;

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let task = args.next().unwrap_or_else(|| "help".to_string());

    match task.as_str() {
        "layers" => layers::run(),
        "stats" => stats::run(),
        "build" => build(args.collect()),
        "ci" => ci(),
        "assets" => assets::run(),
        "perf" => perf::run(),
        "scorecard" => scorecard::run(),
        "help" | "--help" | "-h" => {
            help();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown task: {other}");
            help();
            ExitCode::from(2)
        }
    }
}

fn help() {
    println!("SubstanceCraft workspace tasks:\n");
    println!("  layers         Verify crate layering (a crate depends only on lower layers).");
    println!("  stats          Print crate, source and command counts.");
    println!("  build          Build the release app and copy it into Build/.");
    println!("  ci             Run the full local gate: layers, fmt, clippy, test.");
    println!("  assets         Fetch pinned, permissively licensed test assets into corpus/.");
    println!("  perf           Time representative operations and record perf/baseline.json.");
    println!("  scorecard      Regenerate docs/scorecard.md from the tree and the perf baseline.");
}

/// Build release and copy the finished apps into `Build/`.
fn build(_args: Vec<String>) -> ExitCode {
    if !run_cargo(&["build", "--release"]) {
        return ExitCode::FAILURE;
    }

    let build_dir = std::path::Path::new("Build");
    if let Err(err) = std::fs::create_dir_all(build_dir) {
        eprintln!("could not create Build/: {err}");
        return ExitCode::FAILURE;
    }

    let suffix = std::env::consts::EXE_SUFFIX;
    let binaries = [format!("substancecraft{suffix}"), format!("substancecraft-cli{suffix}")];
    for binary in &binaries {
        let from = std::path::Path::new("target").join("release").join(binary);
        let to = build_dir.join(binary);
        if let Err(err) = std::fs::copy(&from, &to) {
            eprintln!("could not copy {}: {err}", from.display());
            return ExitCode::FAILURE;
        }
        println!("copied {}", to.display());
    }
    ExitCode::SUCCESS
}

/// The full local gate, in the order from AGENTS.md.
fn ci() -> ExitCode {
    if layers::run() != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }
    for args in
        [vec!["fmt", "--all", "--", "--check"], vec!["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"], vec!["test", "--workspace"]]
    {
        if !run_cargo(&args) {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

fn run_cargo(args: &[&str]) -> bool {
    println!("\n$ cargo {}", args.join(" "));
    match Command::new("cargo").args(args).status() {
        Ok(status) if status.success() => true,
        Ok(status) => {
            eprintln!("cargo {} failed with {status}", args.join(" "));
            false
        }
        Err(err) => {
            eprintln!("could not run cargo {}: {err}", args.join(" "));
            false
        }
    }
}
