pub mod cli;
pub mod compare;
pub mod progress;
pub mod report;
pub mod scan;

use anyhow::{Context, Result};
use cli::Args;
use progress::Progress;
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
    let progress = Progress::new(args.progress.into());
    let mut outcome = Outcome::default();

    let started = Instant::now();
    progress.step("scanning A");
    let a = scan::scan(&args.a, |path, size| {
        progress.saw_file(path);
        progress.advance(size);
    })
    .with_context(|| format!("scanning {}", args.a.display()))?;
    progress.step("scanning B");
    let b = scan::scan(&args.b, |path, size| {
        progress.saw_file(path);
        progress.advance(size);
    })
    .with_context(|| format!("scanning {}", args.b.display()))?;
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
                progress.println(format_args!(
                    "{}: missing in B (A has {})",
                    rel.display(),
                    report::size(*size)
                ));
            }
            (None, Some(size)) => {
                outcome.problems += 1;
                progress.println(format_args!(
                    "{}: missing in A (B has {})",
                    rel.display(),
                    report::size(*size)
                ));
            }
            (Some(size_a), Some(size_b)) => {
                if size_a != size_b && args.mode.reports_size() {
                    outcome.problems += 1;
                    progress.println(format_args!(
                        "{}: size differs, {}",
                        rel.display(),
                        report::size_difference(*size_a, *size_b)
                    ));
                } else if args.mode.compares_content() {
                    to_compare.push((rel.to_path_buf(), *size_a));
                } else if args.verbose {
                    progress.println(format_args!(
                        "{}: same size ({})",
                        rel.display(),
                        report::size(*size_a)
                    ));
                }
            }
            (None, None) => unreachable!("relative path comes from at least one tree"),
        }
    }

    if args.mode.compares_content() {
        let jobs = args.jobs.unwrap_or_else(cli::default_jobs);
        let bytes: u64 = to_compare.iter().map(|(_, size)| size).sum();
        progress.step("comparing");
        progress.total(bytes);

        let started = Instant::now();
        let results = compare::compare_all(&to_compare, &args.a, &args.b, jobs, &progress);
        outcome.elapsed_content = started.elapsed().as_secs_f64();
        outcome.compared = results.len();

        for item in results {
            match item.result {
                Ok(true) if args.verbose => {
                    progress.println(format_args!("{}: identical", item.rel.display()))
                }
                Ok(true) => {}
                Ok(false) => {
                    outcome.problems += 1;
                    progress.println(format_args!("{}: content differs", item.rel.display()));
                }
                Err(err) => {
                    outcome.errors += 1;
                    progress.eprintln(format_args!("{}: unreadable: {err}", item.rel.display()));
                }
            }
        }
    }

    progress.finish();
    if outcome.problems == 0 && outcome.errors == 0 {
        progress.println(format_args!("otherwise okay"));
    }
    if args.verbose {
        progress.println(format_args!("{}", summary(&outcome)));
    }

    Ok(outcome)
}

fn summary(outcome: &Outcome) -> String {
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
    line
}
