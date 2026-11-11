use std::path::PathBuf;
use clap::Parser;

/// Compare two directory trees and print only what differs.
#[derive(Parser, Debug)]
#[command(name = "dirverify", version, about)]
pub struct Args {
    /// Reference tree, reported as "A".
    pub a: PathBuf,

    /// Tree verified against A, reported as "B".
    pub b: PathBuf,

    /// Also print files that match.
    #[arg(short, long)]
    pub verbose: bool,
}
