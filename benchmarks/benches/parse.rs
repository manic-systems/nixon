use std::{
    io::Write,
    process::{Command, Stdio},
};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

use nixon_benchmarks::cases;

/// Parses `source` with the reference C++ parser, `nix-instantiate --parse`,
/// piping the input through stdin so file I/O stays outside the timed region.
fn nix_instantiate(source: &str) {
    let mut child = Command::new("nix-instantiate")
        .args(["--parse", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn `nix-instantiate`; is it installed?");
    let mut stdin = child.stdin.take().expect("`nix-instantiate` stdin is piped");
    stdin
        .write_all(source.as_bytes())
        .expect("failed to write to `nix-instantiate` stdin");
    drop(stdin);
    let status = child.wait().expect("failed to wait for `nix-instantiate`");
    assert!(status.success(), "`nix-instantiate --parse` failed");
}

/// Returns whether `nix-instantiate` is available on `PATH`.
fn nix_instantiate_available() -> bool {
    Command::new("nix-instantiate")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn benchmarks(criterion: &mut Criterion) {
    let reference = nix_instantiate_available();

    for case in cases() {
        let mut group = criterion.benchmark_group(case.name);
        group.throughput(Throughput::Bytes(case.source.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("nixon", case.source.len()),
            &case.source,
            |bencher, source| {
                bencher.iter(|| {
                    std::hint::black_box(nixon::parse_syntax(std::hint::black_box(source)))
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("rnix", case.source.len()),
            &case.source,
            |bencher, source| {
                bencher.iter(|| rnix::Root::parse(std::hint::black_box(source)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("nixel", case.source.len()),
            &case.source,
            |bencher, source| {
                bencher.iter(|| nixel::parse(std::hint::black_box(source.clone())));
            },
        );
        if reference {
            group.bench_with_input(
                BenchmarkId::new("nix-instantiate", case.source.len()),
                &case.source,
                |bencher, source| {
                    bencher.iter(|| nix_instantiate(std::hint::black_box(source)));
                },
            );
        }
        group.finish();
    }

    if !reference {
        eprintln!(
            "warning: `nix-instantiate` was not found on PATH; its benchmark was \
             skipped"
        );
    }
}

criterion_group!(parser_benches, benchmarks);
criterion_main!(parser_benches);
