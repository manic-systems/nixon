# Benchmarks

This is the benchmark crate for comparing Nixon against fixes revisions of rnix
and NixEL. At the time of this benchmarked, the revisions for the compared
targets are as follows:

- Nixon: `c272c29eca7dc7f6b91409c70e7352f6d1bbacbe`
- rnix: `aae4163d88efefcba5e11482f25a5e6dcd21492c`
- NixEL: `a4d7ccfd2a5ce28b6ffdc2ed0dd3f6c339b2357f`

While you will not get _identical_ results from the revisions alone (obviously),
they should help you asses what exactly was benchmarked. The benchmark crate is
GPL-3.0-only and lives in its own workspace because it links NixEL. I'm not sure
if this entails any other license obligations.

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

Each timed call builds a complete syntax tree, with file I/O being excluded.
Nixon uses `parse_syntax`; `parse` also runs semantic validation that the other
parser APIs do not provide. NixEL consumes a `String`, so its timed call
includes the required clone.

The `nix-instantiate` adapter is the reference C++ parser spawned as a
subprocess (`nix-instantiate --parse -`) with input piped through stdin, so its
timing includes process startup, and `--parse` also checks for unbound
variables. Its benchmark is skipped when `nix-instantiate` is not on `PATH`.

## Results

The latest recording of the benchmarking results are from **26.08.24**. This
section will be updated as more benchmarks are ran when either upstream updates,
a new "competitor" is added, or when Nixon receives a meaningful change to the
parser logic.

- [2026-08-24 results](results/2026-08-24.md)
- [2026-08-23 results](results/2026-08-23.md)
