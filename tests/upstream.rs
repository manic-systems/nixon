//! Differential tests against a separately pinned Nix source checkout.

use std::{env, fs, path::PathBuf};

use nixon::{ParseOptions, UrlLiteralPolicy, parse, parse_with_options};

fn language_tests() -> PathBuf {
    env::var_os("NIXON_NIX_LANGUAGE_TESTS")
        .map(PathBuf::from)
        .expect("set NIXON_NIX_LANGUAGE_TESTS to Nix's tests/functional/lang directory")
}

fn rnix_tests() -> PathBuf {
    env::var_os("NIXON_RNIX_TESTS")
        .map(PathBuf::from)
        .expect("set NIXON_RNIX_TESTS to rnix-parser's test_data/parser/success directory")
}

#[test]
#[ignore = "requires a pinned external Nix source checkout"]
fn accepts_upstream_parse_okay_fixtures() {
    let directory = language_tests();
    let mut failures = Vec::new();
    for entry in fs::read_dir(&directory).expect("read Nix language test directory") {
        let path = entry.expect("read directory entry").path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("parse-okay-")
            || path.extension().and_then(|ext| ext.to_str()) != Some("nix")
        {
            continue;
        }
        let source = fs::read_to_string(&path).expect("upstream fixture is UTF-8");
        let document = parse(&source).expect("upstream fixture fits compact offsets");
        if !document.is_valid() {
            failures.push((name.to_owned(), document.diagnostics().to_vec()));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
#[ignore = "requires a pinned external Nix source checkout"]
fn rejects_upstream_parse_fail_fixtures() {
    let directory = language_tests();
    let mut failures = Vec::new();
    for entry in fs::read_dir(&directory).expect("read Nix language test directory") {
        let path = entry.expect("read directory entry").path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("parse-fail-")
            || path.extension().and_then(|ext| ext.to_str()) != Some("nix")
        {
            continue;
        }
        let source = fs::read_to_string(&path).expect("upstream fixture is UTF-8");
        let document = parse(&source).expect("upstream fixture fits compact offsets");
        if document.is_valid() {
            failures.push(name.to_owned());
        }
    }
    assert!(
        failures.is_empty(),
        "accepted invalid fixtures: {failures:#?}"
    );
}

#[test]
#[ignore = "requires a separately pinned rnix source checkout"]
fn accepts_rnix_success_corpus() {
    let directory = rnix_tests();
    let options = ParseOptions {
        pipe_operators: true,
        uri_literals: UrlLiteralPolicy::Allow,
        validate_identifiers: false,
        ..ParseOptions::default()
    };
    let mut failures = Vec::new();
    for entry in fs::read_dir(&directory).expect("read rnix parser test directory") {
        let path = entry.expect("read directory entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("nix") {
            continue;
        }
        let name = path.file_name().and_then(|name| name.to_str());
        if matches!(
            name,
            Some(
                "inherit_dynamic.nix"
                    | "pipe_mixed.nix"
                    | "pipe_mixed_math.nix"
                    | "uri_various.nix"
            )
        ) {
            // These rnix extensions are rejected by the pinned Nix 2.35 grammar.
            continue;
        }
        let source = fs::read_to_string(&path).expect("rnix fixture is UTF-8");
        let document = parse_with_options(&source, options).expect("fixture fits compact offsets");
        if !document.is_valid() {
            failures.push((path, document.diagnostics().to_vec()));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
