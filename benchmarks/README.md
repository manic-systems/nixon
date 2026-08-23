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

The timed code excludes file I/O and keeps each complete parse result alive.
NixEL takes ownership of its input, so its timed call includes the required
`String` clone.

## Binaries

The crate also provides one small adapter for each parser:

```sh
# Build the stripped release adapters.
$ cargo build --manifest-path benchmarks/Cargo.toml --release --bins
```

Each adapter accepts `tiny`, `interpolation`, `module`, `large`, or a path to a
Nix file.
