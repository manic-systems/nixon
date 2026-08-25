use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use nixon_benchmarks::cases;

fn benchmarks(criterion: &mut Criterion) {
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
        group.finish();
    }
}

criterion_group!(parser_benches, benchmarks);
criterion_main!(parser_benches);
