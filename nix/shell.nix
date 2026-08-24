{
  mkShell,
  rustc,
  cargo,
  rust-analyzer,
  rustfmt,
  clippy,
  taplo,
  lib,
  llvmPackages,
  binaryen,
  hyperfine,
  valgrind,
  wasm-bindgen-cli,
}:
mkShell {
  name = "nixon";

  strictDeps = true;
  nativeBuildInputs = [
    rustc
    cargo

    # Tools
    (rustfmt.override {asNightly = true;})
    clippy
    taplo
    binaryen
    hyperfine
    valgrind
    wasm-bindgen-cli

    # NixEL's generated C++ binding.
    llvmPackages.libclang
    llvmPackages.lld

    # LSP
    rust-analyzer
  ];

  env.LIBCLANG_PATH = lib.makeLibraryPath [llvmPackages.libclang.lib];
}
