# Nixon

A small, lossless parser for the Nix language, written in Rust.

Nixon is a neat little parser that keeps comments and whitespace in the syntax
tree, recovers from bad syntax, and performs the parse-time checks used by Nix
2.35. The parser crate depends on _nothing_. It also forbids unsafe Rust because
there is simply no good reason for a parser to need it. Though I might
_eventually_ add a tiny bit of assembly in there. As a treat.

For now, the API is at 0.1 and may change when it annoys me. The parser itself
is solid enough that it can be considered more than just a prototype, but it
should _not_ be considered stable either. It is tested against the Nix language
fixtures, the shared rnix fixtures, fuzzed tree invariants, and the native and
WebAssembly APIs. Oh yeah, we can do WebAssembly.

## Motivation

[rnix]: https://github.com/nix-community/rnix-parser
[NixEL]: https://github.com/kamadorueda/nixel

If you are wondering "why another parser", well the answer is quite obvious, but
I'll give you two reasons:

1. I wanted to, duh
2. I wanted to go faster _and_ smaller than both

Ultimately I ~~wanted~~ required a parser that I could use in native tools and
ship to a browser without dragging a large dependency graph behind it so that I
can build a _small_ Nix formatter. [rnix] and [NixEL] are the (obvious) prior
art from which Nixon is inspired, but they do not quite measure up to what I had
expected.

## Usage

`parse` builds the syntax tree and runs Nix's parse-time validation:

```rust
use nixon::{Element, parse};

let source = "let answer = 40 + 2; in answer";
let document = parse(source)?;

assert!(document.is_valid());
assert_eq!(document.root().text(), source);

for element in document.root().children() {
    match element {
        Element::Node(node) => println!("{:?} {}", node.kind(), node.text()),
        Element::Token(token) => println!("{:?} {}", token.kind(), token.text()),
    }
}
# Ok::<(), nixon::InputError>(())
```

Use `parse_syntax` if you only need the lossless tree. `parse_with_options`
controls experimental pipe operators, URI literals, identifier validation, and
additional names in the root scope. Typed nodes live under `nixon::ast`; the
untyped `Node`, `TokenNode`, and `ElementId` API is useful for editors and
language bindings.

### WebAssembly

The `nixon-wasm` crate owns its input and exposes the tree as integer-indexed
records. This avoids building thousands of short-lived JavaScript objects just
to cross the Wasm boundary.

```sh
# Compile the WebAssembly crate.
$ cargo build -p nixon-wasm --target wasm32-unknown-unknown --release

# Generate bindings for web browsers.
$ wasm-bindgen target/wasm32-unknown-unknown/release/nixon_wasm.wasm \
    --target web --out-dir dist

# Optimize the generated WebAssembly binary.
$ wasm-opt -Oz --enable-bulk-memory dist/nixon_wasm_bg.wasm \
    -o dist/nixon_wasm_bg.wasm
```

The generated module exports `parse`, `parseWithOptions`, and `ParsedNix`. The
optimized browser binary from the recorded run is 114,567 bytes, or 53,192 bytes
after gzip. Which is pretty modest, but a good start.

### Limits

Nixon's speed and size are not without some... drawbacks.

- Ranges are UTF-8 byte offsets.
- Input size is limited to 4 GiB by the 32-bit offsets.
- A tree may contain up to 16,777,215 nodes and tokens.
- Recursive expression nesting stops at 256 and produces a diagnostic. Flat
  lists, sets, and function applications do not spend that budget by themselves.

I'll try to resolve each one of those individually, though, it's not like they
should affect everyday usage.

## Benchmarks

[repository benchmarks]: benchmarks/
[2026-08-23 results]: benchmarks/results/2026-08-23.md

The comparison crate pins rnix 0.14.0 and NixEL 5.1.1. On my Ryzen 7 7700X,
Nixon parsed the 251,750-byte input in 5.93 ms. rnix took 12.25 ms and NixEL
took 55.46 ms. Nixon also produced the smallest stripped adapter and used the
least peak heap in that test.

Take a look at the [repository benchmarks] and the initial benchmarking run
recorded in [2026-08-23 results] with some additional notes on input generator,
adapters, pinned rev, commands, and complete results with more statistics you
might be interested in.

## Hacking

The flake provides Cargo, rust-analyzer, the Wasm tools, benchmark dependencies,
and fuzzing tools. `nix develop` is the least annoying way to get all of them.

```sh
# Run the native test suite.
$ cargo test --workspace

# Lint every native target.
$ cargo clippy --workspace --all-targets -- -D warnings

# Check the browser target.
$ cargo check -p nixon-wasm --target wasm32-unknown-unknown

# Check the fuzzing crate.
$ cargo check --manifest-path fuzz/Cargo.toml --all-targets
```

You can also run `nix flake check` for good measure if you'd like. Though it
doesn't do anything meaningful. Yet. I'll probably consider VM or container
tests in the future for testing the web? Not sure yet.

### Testing

The external parser fixtures are not vendored. I didn't want to think about
licensing, so I'm letting you all the fun. Set `NIXON_NIX_LANGUAGE_TESTS` and
`NIXON_RNIX_TESTS` to their respective checkout paths before running the
upstream tests. [`tests/upstream.rs`](tests/upstream.rs) documents the expected
layout for you.

## License

[provided here]: https://interoperable-europe.ec.europa.eu/sites/default/files/custom-page/attachment/eupl_v1.2_en.pdf

Nixon is made available under European Union Public Licence (EUPL) version 1.2.
See [LICENSE](LICENSE) for more details on the exact conditions. An online copy
is provided [provided here].
