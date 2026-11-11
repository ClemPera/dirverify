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
    /// Compare file sizes only.
    Size,
    /// Compare file contents only.
    Content,
    /// Compare sizes, then the contents of equal-size pairs.
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
    /// Live line on a terminal, silent otherwise.
    Auto,
    /// Live line on a terminal, periodic lines otherwise.
    Always,
    /// Do not report progress.
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
