use clap::Parser;
use dirverify::cli::Args;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = Args::parse();
    match dirverify::run(&args) {
        Ok(outcome) if outcome.problems == 0 && outcome.errors == 0 => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(1),
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}
