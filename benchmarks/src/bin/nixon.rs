use std::{env, hint::black_box, process::ExitCode};

use nixon_benchmarks::load;

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: nixon <tiny|interpolation|module|large|file>");
        return ExitCode::FAILURE;
    };
    let source = match load(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    match nixon::parse_syntax(black_box(&source)) {
        Ok(document) => {
            black_box(document);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
