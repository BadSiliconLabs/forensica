//! `forensica-cli`: headless front-end to the Forensica engine (offline).
//!
//! ```text
//! forensica-cli analyze <file> [--json <out.json>] [--max-mp <N>]
//! forensica-cli batch <input-dir> <output-dir> [--max-mp <N>]
//! forensica-cli bench <file> [--runs <N>]
//! forensica-cli samples <output-dir> [--big]
//! ```

mod samples;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use forensica_core::{CancelToken, ImageInput, Options, Report, Severity};

fn usage() -> ExitCode {
    eprintln!(
        "forensica-cli {} (offline photo forensics)\n\n\
         USAGE:\n  forensica-cli analyze <file> [--json <out.json>] [--max-mp <N>]\n  \
         forensica-cli batch <input-dir> <output-dir> [--max-mp <N>]\n  \
         forensica-cli bench <file> [--runs <N>]\n  \
         forensica-cli samples <output-dir> [--big]",
        forensica_core::ENGINE_VERSION
    );
    ExitCode::from(2)
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn options(args: &[String]) -> Options {
    let mp: u64 = flag(args, "--max-mp").and_then(|v| v.parse().ok()).unwrap_or(0);
    Options { max_pixels: mp * 1_000_000, ..Options::default() }
}

fn analyze_file(path: &Path, opts: &Options) -> Result<Report, String> {
    let input = ImageInput::from_path(path).map_err(|e| e.to_string())?;
    Ok(forensica_engine::pipeline().run(&input, opts, &CancelToken::new(), &|_| {}))
}

fn print_report(r: &Report) {
    println!("{}  sha256={}  {:?}  {} ms", r.file.name, r.file.sha256, r.file.format, r.duration_ms);
    for f in r.findings() {
        let tag = match f.severity {
            Severity::High => "HIGH  ",
            Severity::Medium => "MEDIUM",
            Severity::Low => "low   ",
            Severity::Info => "info  ",
        };
        println!("  [{tag}] {:>3}%  {}  ({})", (f.confidence * 100.0).round(), f.title, f.code);
    }
    for m in r.modules.iter().filter(|m| m.error.is_some()) {
        println!("  [error ] {}: {}", m.id, m.error.as_deref().unwrap_or_default());
    }
}

fn cmd_analyze(args: &[String]) -> ExitCode {
    let Some(file) = args.first() else { return usage() };
    match analyze_file(Path::new(file), &options(args)) {
        Ok(r) => {
            print_report(&r);
            if let Some(out) = flag(args, "--json") {
                if let Err(e) = std::fs::write(&out, r.to_json()) {
                    eprintln!("cannot write {out}: {e}");
                    return ExitCode::FAILURE;
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{file}: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_batch(args: &[String]) -> ExitCode {
    let (Some(inp), Some(out)) = (args.first(), args.get(1)) else { return usage() };
    let opts = options(args);
    let Ok(rd) = std::fs::read_dir(inp) else {
        eprintln!("cannot read {inp}");
        return ExitCode::FAILURE;
    };
    if let Err(e) = std::fs::create_dir_all(out) {
        eprintln!("cannot create {out}: {e}");
        return ExitCode::FAILURE;
    }
    let mut files: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_file()).collect();
    files.sort();
    let (mut ok, mut failed) = (0, 0);
    for p in files {
        match analyze_file(&p, &opts) {
            Ok(r) => {
                print_report(&r);
                let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let dest = Path::new(out).join(format!("{name}.forensica.json"));
                if std::fs::write(&dest, r.to_json()).is_ok() {
                    ok += 1;
                } else {
                    failed += 1;
                }
            }
            Err(e) => {
                eprintln!("{}: {e}", p.display());
                failed += 1;
            }
        }
    }
    println!("done: {ok} analysed, {failed} failed");
    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn cmd_bench(args: &[String]) -> ExitCode {
    let Some(file) = args.first() else { return usage() };
    let runs: usize = flag(args, "--runs").and_then(|v| v.parse().ok()).unwrap_or(1);
    for max_mp in [0u64, 12] {
        for _ in 0..runs {
            forensica_engine::pipeline().clear_cache();
            let t0 = Instant::now();
            let input = match ImageInput::from_path(Path::new(file)) {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("{file}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let t_read = t0.elapsed();
            let decoded = input.rgb().map(|i| i.dimensions());
            let t_decode = t0.elapsed() - t_read;
            let opts = Options { max_pixels: max_mp * 1_000_000, ..Options::default() };
            let r = forensica_engine::pipeline().run(&input, &opts, &CancelToken::new(), &|_| {});
            println!(
                "max_mp={max_mp:<3} total={:>6} ms  (read+sha {} ms, decode {} ms, pipeline {} ms)  dims={:?}",
                t0.elapsed().as_millis(),
                t_read.as_millis(),
                t_decode.as_millis(),
                r.duration_ms,
                decoded.ok()
            );
            let mut mods: Vec<_> = r.modules.iter().collect();
            mods.sort_by_key(|m| std::cmp::Reverse(m.duration_ms));
            for m in mods {
                println!("    {:<14} {:>6} ms {}", m.id, m.duration_ms, m.error.as_deref().unwrap_or(""));
            }
        }
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("analyze") => cmd_analyze(&args[1..]),
        Some("batch") => cmd_batch(&args[1..]),
        Some("bench") => cmd_bench(&args[1..]),
        Some("samples") => match args.get(1) {
            Some(dir) if args.iter().any(|a| a == "--big") => match samples::write_big(Path::new(dir)) {
                Ok(()) => {
                    println!("wrote {dir}/24mp.jpg");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            },
            Some(dir) => match samples::write_all(Path::new(dir)) {
                Ok(n) => {
                    println!("wrote {n} sample files to {dir}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            },
            None => usage(),
        },
        _ => usage(),
    }
}
