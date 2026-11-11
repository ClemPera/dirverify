//! Verify that two directory trees are identical.
//!
//! The size pass compares relative paths and byte sizes without opening any file;
//! the content pass then compares same-size pairs byte by byte in parallel.

pub mod cli;
pub mod compare;
pub mod progress;
pub mod report;
pub mod scan;

use anyhow::{Context, Result};
use cli::Args;
use progress::Progress;
use std::time::Instant;

#[derive(Default)]
pub struct Outcome {
    pub problems: usize,
    pub errors: usize,
    pub files_a: usize,
    pub files_b: usize,
    pub bytes_a: u64,
    pub bytes_b: u64,
    pub compared: usize,
    pub bytes_compared: u64,
    pub skipped: usize,
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
    outcome.bytes_a = a.files.values().sum();
    outcome.bytes_b = b.files.values().sum();
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
                    "{}: missing from B, present in A ({})",
                    rel.display(),
                    report::size(*size)
                ));
            }
            (None, Some(size)) => {
                outcome.problems += 1;
                progress.println(format_args!(
                    "{}: missing from A, present in B ({})",
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
        outcome.bytes_compared = to_compare.iter().map(|(_, size)| size).sum();

        progress.step("comparing");
        progress.total(outcome.bytes_compared);

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
    if args.verbose {
        progress.println(format_args!("{}", summary(&outcome)));
    }

    Ok(outcome)
}

fn summary(outcome: &Outcome) -> String {
    let mut lines = vec![
        format!(
            "A:        {} {}, {}",
            report::count(outcome.files_a as u64),
            report::plural(outcome.files_a as u64),
            report::size(outcome.bytes_a)
        ),
        format!(
            "B:        {} {}, {}",
            report::count(outcome.files_b as u64),
            report::plural(outcome.files_b as u64),
            report::size(outcome.bytes_b)
        ),
    ];
    if outcome.compared > 0 {
        lines.push(format!(
            "compared: {} {}, {}",
            report::count(outcome.compared as u64),
            report::plural(outcome.compared as u64),
            report::size(outcome.bytes_compared)
        ));
    }
    if outcome.skipped > 0 {
        lines.push(format!("skipped:  {} entries", outcome.skipped));
    }
    lines.push(format!(
        "elapsed:  {:.1}s",
        outcome.elapsed_scan + outcome.elapsed_content
    ));
    lines.push(format!("result:   {}", result(outcome)));
    lines.join("\n")
}

fn result(outcome: &Outcome) -> String {
    let mut text = match outcome.problems {
        0 => "no differences".to_string(),
        1 => "1 difference".to_string(),
        other => format!("{} differences", report::count(other as u64)),
    };
    if outcome.errors > 0 {
        text.push_str(&format!(
            ", {} unreadable",
            report::count(outcome.errors as u64)
        ));
    }
    text
}
