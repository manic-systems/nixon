# Fuzzing

Run the losslessness, range, and tree-link invariant target from `fuzz/`:

```sh
# Check the syntax tree invariants.
$ cargo fuzz run parse
```

The fuzz crate has its own Cargo workspace because only it needs
`libfuzzer-sys`.
