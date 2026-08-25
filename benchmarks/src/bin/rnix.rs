use std::{env, hint::black_box, process::ExitCode};

use nixon_benchmarks::load;

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: rnix <tiny|interpolation|module|large|file>");
        return ExitCode::FAILURE;
    };
    let source = match load(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    black_box(rnix::Root::parse(black_box(&source)));
    ExitCode::SUCCESS
}
