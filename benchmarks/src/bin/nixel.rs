use std::{env, fs, hint::black_box, process::ExitCode};

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: nixel <file>");
        return ExitCode::FAILURE;
    };
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    black_box(nixel::parse(black_box(source)));
    ExitCode::SUCCESS
}
