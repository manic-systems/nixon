{
  lib,
  rustPlatform,
  stdenv,
}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "nixon";
  version = "0.1.0";
  __structuredAttrs = true;

  src = let
    fs = lib.fileset;
    s = ../.;
  in
    fs.toSource {
      root = s;
      fileset = fs.unions [
        (s + /crates/ffi)
        (s + /crates/nixon)
        (s + /crates/wasm)

        (s + /Cargo.lock)
        (s + /Cargo.toml)
        (s + /README.md)
      ];
    };

  cargoLock.lockFile = "${finalAttrs.src}/Cargo.lock";
  cargoBuildFlags = ["-p" "nixon-ffi"];
  cargoTestFlags = ["-p" "nixon-ffi"];
  enableParallelBuilding = true;

  installPhase = ''
    runHook preInstall
    install -Dm644 crates/ffi/include/nixon.h $out/include/nixon.h
    install -Dm644 target/${stdenv.hostPlatform.rust.cargoShortTarget}/release/libnixon_ffi.a \
      $out/lib/libnixon_ffi.a
    install -Dm755 target/${stdenv.hostPlatform.rust.cargoShortTarget}/release/libnixon_ffi${stdenv.hostPlatform.extensions.sharedLibrary} \
      $out/lib/libnixon_ffi${stdenv.hostPlatform.extensions.sharedLibrary}
    runHook postInstall
  '';

  meta = {
    description = "Compact lossless parser for the Nix language";
    homepage = "https://github.com/manic-systems/nixon";
    license = lib.licenses.eupl12;
    maintainers = with lib.maintainers; [NotAShelf];
    platforms = lib.platforms.all;
  };
})
