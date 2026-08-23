{
  mkShell,
  rustc,
  cargo,
  rust-analyzer,
  rustfmt,
  clippy,
  taplo,
}:
mkShell {
  name = "rust";

  strictDeps = true;
  nativeBuildInputs = [
    rustc
    cargo

    # Tools
    rustfmt
    clippy
    cargo
    taplo

    # LSP
    rust-analyzer
  ];
}
