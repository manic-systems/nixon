use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use nixon::{ParseOptions, UrlLiteralPolicy};
use nixon_competitors::cases;

fn benchmarks(criterion: &mut Criterion) {
    for case in cases() {
        let mut group = criterion.benchmark_group(case.name);
        group.throughput(Throughput::Bytes(case.source.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("nixon", case.source.len()),
            &case.source,
            |bencher, source| {
                let options = ParseOptions {
                    uri_literals: UrlLiteralPolicy::Allow,
                    validate_identifiers: false,
                    ..ParseOptions::default()
                };
                bencher.iter(|| {
                    std::hint::black_box(nixon::parse_with_options(
                        std::hint::black_box(source),
                        options,
                    ))
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
        group.finish();
    }
}

criterion_group!(parser_benches, benchmarks);
criterion_main!(parser_benches);
