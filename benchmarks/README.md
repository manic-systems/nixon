# Benchmarks

The benchmark crate compares Nixon against fixed revisions of rnix and NixEL:

- Nixon: the current checkout
- rnix: `aae4163d88efefcba5e11482f25a5e6dcd21492c`
- NixEL: `a4d7ccfd2a5ce28b6ffdc2ed0dd3f6c339b2357f`

The benchmark crate is GPL-3.0-only and lives in its own workspace because it
links NixEL.

## Throughput

Run the benchmark from the repository root:

```sh
# Compare the parsers across all four inputs.
$ cargo bench --manifest-path benchmarks/Cargo.toml --bench parse
```

## Binary size

```sh
# Build one stripped release binary for each parser.
$ cargo build --manifest-path benchmarks/Cargo.toml --release --bins
```

The binaries accept `tiny`, `interpolation`, `module`, `large`, or a path to a
Nix file.

## Peak heap

```sh
# Profile Nixon while it parses the generated large input.
$ valgrind --tool=massif --massif-out-file=nixon.massif \
    benchmarks/target/release/nixon large
```

Replace `nixon` with `rnix` or `nixel` to profile the other parsers.

## Method

Each timed call builds a complete syntax tree. File I/O is excluded. Nixon uses
`parse_syntax`; `parse` also runs semantic validation that the other parser APIs
do not provide. NixEL consumes a `String`, so its timed call includes the
required clone.

## Results

See the recorded [2026-08-23 results](results/2026-08-23.md).
