use clap::{Parser, ValueEnum};
use std::path::PathBuf;

use crate::progress::When;

/// Compare two directory trees and print only what differs.
#[derive(Parser, Debug)]
#[command(name = "dirverify", version, about)]
pub struct Args {
    /// Reference tree, reported as "A".
    pub a: PathBuf,

    /// Tree verified against A, reported as "B".
    pub b: PathBuf,

    /// Which passes to run.
    #[arg(short, long, value_enum, default_value_t = Mode::Both)]
    pub mode: Mode,

    /// Threads for the byte-exact pass.
    #[arg(short, long)]
    pub jobs: Option<usize>,

    /// Progress display on stderr.
    #[arg(long, value_enum, default_value_t = Progress::Auto)]
    pub progress: Progress,

    /// Also print files that match, plus per-pass timings.
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Mode {
    /// Sizes only; fast but blind to same-size corruption.
    Size,
    /// Bytes only; reports content differences even for uneven sizes.
    Content,
    /// Sizes first, then bytes for every pair of equal size.
    Both,
}

impl Mode {
    pub fn reports_size(self) -> bool {
        matches!(self, Mode::Size | Mode::Both)
    }

    pub fn compares_content(self) -> bool {
        matches!(self, Mode::Content | Mode::Both)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Progress {
    /// Live line when stderr is a terminal, silent otherwise.
    Auto,
    /// Always report: live line on a terminal, periodic lines in a pipe.
    Always,
    /// Never report.
    Never,
}

impl From<Progress> for When {
    fn from(value: Progress) -> Self {
        match value {
            Progress::Auto => When::Auto,
            Progress::Always => When::Always,
            Progress::Never => When::Never,
        }
    }
}

pub fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
