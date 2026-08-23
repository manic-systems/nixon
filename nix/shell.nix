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
  alejandra,
}:
mkShell {
  name = "nixon";

  strictDeps = true;
  nativeBuildInputs = [
    rustc
    cargo

    # Tools
    rustfmt
    clippy
    cargo
    taplo
    binaryen
    hyperfine
    valgrind
    wasm-bindgen-cli
    alejandra

    # NixEL's generated C++ binding.
    llvmPackages.libclang

    # LSP
    rust-analyzer
  ];

  LIBCLANG_PATH = lib.makeLibraryPath [llvmPackages.libclang.lib];
}
