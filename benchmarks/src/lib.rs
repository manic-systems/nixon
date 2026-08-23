//! Inputs shared by the parser benchmarks.

use std::fmt::Write;

/// One fixed benchmark input.
pub struct Case {
    /// Benchmark name.
    pub name: &'static str,
    /// Nix source given to each parser.
    pub source: String,
}

/// Returns the built-in benchmark inputs.
pub fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "tiny",
            source: "let x = 1; in x + 2".to_owned(),
        },
        Case {
            name: "interpolation",
            source: interpolation_case(),
        },
        Case {
            name: "module",
            source: module_case(128),
        },
        Case {
            name: "large",
            source: module_case(4_096),
        },
    ]
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
