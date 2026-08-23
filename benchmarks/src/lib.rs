//! Inputs shared by the parser benchmarks.

use std::{ffi::OsStr, fmt::Write, fs, io};

/// One fixed benchmark input.
pub struct Case {
    /// Benchmark name.
    pub name: &'static str,
    /// Nix source given to each parser.
    pub source: String,
}

/// Returns the built-in benchmark inputs.
pub fn cases() -> Vec<Case> {
    ["tiny", "interpolation", "module", "large"]
        .into_iter()
        .map(|name| case(name).expect("known benchmark case"))
        .collect()
}

/// Returns a built-in benchmark input by name.
pub fn case(name: &str) -> Option<Case> {
    Some(match name {
        "tiny" => Case {
            name: "tiny",
            source: "let x = 1; in x + 2".to_owned(),
        },
        "interpolation" => Case {
            name: "interpolation",
            source: interpolation_case(),
        },
        "module" => Case {
            name: "module",
            source: module_case(128),
        },
        "large" => Case {
            name: "large",
            source: module_case(4_096),
        },
        _ => return None,
    })
}

/// Loads a named benchmark case or a UTF-8 file path.
pub fn load(argument: &OsStr) -> io::Result<String> {
    if let Some(case) = argument.to_str().and_then(case) {
        Ok(case.source)
    } else {
        fs::read_to_string(argument)
    }
}

fn interpolation_case() -> String {
    r#"
let
  user = "nixon";
  root = ./packages/${user}/src;
in {
  message = ''
    parser: ${user}
    root: ${toString root}
    escaped: ''${literal} and ''\n
  '';
  paths = [ root ./tests/${user}/cases <nixpkgs> ];
}
"#
    .to_owned()
}

fn module_case(bindings: usize) -> String {
    let mut source = String::with_capacity(bindings * 48);
    source.push_str("{ lib, ... }: {\n");
    for index in 0..bindings {
        writeln!(
            source,
            "  services.nixon.item{index} = {{ enable = true; value = {index}; }};"
        )
        .expect("writing to String cannot fail");
    }
    source.push_str("}\n");
    source
}
