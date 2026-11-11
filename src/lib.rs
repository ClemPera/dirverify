pub mod cli;
pub mod compare;
pub mod report;
pub mod scan;

use anyhow::{Context, Result};
use cli::Args;
use std::fmt::Write as _;
use std::time::Instant;

#[derive(Default)]
pub struct Outcome {
    pub problems: usize,
    pub errors: usize,
    pub files_a: usize,
    pub files_b: usize,
    pub skipped: usize,
    pub compared: usize,
    pub elapsed_scan: f64,
    pub elapsed_content: f64,
}

pub fn run(args: &Args) -> Result<Outcome> {
    let mut outcome = Outcome::default();

    let started = Instant::now();
    let a = scan::scan(&args.a).with_context(|| format!("scanning {}", args.a.display()))?;
    let b = scan::scan(&args.b).with_context(|| format!("scanning {}", args.b.display()))?;
    outcome.elapsed_scan = started.elapsed().as_secs_f64();
    outcome.files_a = a.files.len();
    outcome.files_b = b.files.len();
    outcome.skipped = a.skipped + b.skipped;

    let mut all = Vec::with_capacity(a.files.len() + b.files.len());
    all.extend(a.files.keys());
    all.extend(b.files.keys());
    all.sort();
    all.dedup();

    let mut to_compare = Vec::new();
    for rel in all {
        match (a.files.get(rel), b.files.get(rel)) {
            (Some(size), None) => {
                outcome.problems += 1;
                println!(
                    "{}: missing in B (A has {})",
                    rel.display(),
                    report::size(*size)
                );
            }
            (None, Some(size)) => {
                outcome.problems += 1;
                println!(
                    "{}: missing in A (B has {})",
                    rel.display(),
                    report::size(*size)
                );
            }
            (Some(size_a), Some(size_b)) => {
                if size_a != size_b && args.mode.reports_size() {
                    outcome.problems += 1;
                    println!(
                        "{}: size differs, {}",
                        rel.display(),
                        report::size_difference(*size_a, *size_b)
                    );
                } else if args.mode.compares_content() {
                    to_compare.push(rel.to_path_buf());
                } else if args.verbose {
                    println!("{}: same size ({})", rel.display(), report::size(*size_a));
                }
            }
            (None, None) => unreachable!("relative path comes from at least one tree"),
        }
    }

    if args.mode.compares_content() {
        let jobs = args.jobs.unwrap_or_else(cli::default_jobs);
        let started = Instant::now();
        let results = compare::compare_all(&to_compare, &args.a, &args.b, jobs);
        outcome.elapsed_content = started.elapsed().as_secs_f64();
        outcome.compared = results.len();

        for item in results {
            match item.result {
                Ok(true) if args.verbose => println!("{}: identical", item.rel.display()),
                Ok(true) => {}
                Ok(false) => {
                    outcome.problems += 1;
                    println!("{}: content differs", item.rel.display());
                }
                Err(err) => {
                    outcome.errors += 1;
                    eprintln!("{}: unreadable: {err}", item.rel.display());
                }
            }
        }
    }

    if outcome.problems == 0 && outcome.errors == 0 {
        println!("otherwise okay");
    }
    if args.verbose {
        print_summary(&outcome);
    }

    Ok(outcome)
}

fn print_summary(outcome: &Outcome) {
    let mut line = String::new();
    write!(
        line,
        "A: {} files, B: {} files, {} compared byte-for-byte",
        outcome.files_a, outcome.files_b, outcome.compared
    )
    .unwrap();
    write!(line, ", scan {:.2}s", outcome.elapsed_scan).unwrap();
    if outcome.skipped > 0 {
        write!(line, ", {} skipped", outcome.skipped).unwrap();
    }
    if outcome.elapsed_content > 0.0 {
        write!(line, ", compare {:.2}s", outcome.elapsed_content).unwrap();
    }
    println!("{line}");
}
