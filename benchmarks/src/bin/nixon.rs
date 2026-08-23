use std::{env, fs, hint::black_box, process::ExitCode};

use nixon::{ParseOptions, UrlLiteralPolicy};

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: nixon <file>");
        return ExitCode::FAILURE;
    };
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let options = ParseOptions {
        uri_literals: UrlLiteralPolicy::Allow,
        validate_identifiers: false,
        ..ParseOptions::default()
    };
    match nixon::parse_with_options(black_box(&source), options) {
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
