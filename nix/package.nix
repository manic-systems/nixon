{
  lib,
  rustPlatform,
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
        (s + /src)
        (s + /ffi)
        (s + /wasm)
        (s + /tests)

        (s + /Cargo.lock)
        (s + /Cargo.toml)
        (s + /README.md)
      ];
    };

  cargoLock.lockFile = "${finalAttrs.src}/Cargo.lock";
  enableParallelBuilding = true;

  meta = {
    description = "Compact lossless parser for the Nix language";
    homepage = "https://github.com/manic-systems/nixon";
    license = lib.licenses.eupl12;
    maintainers = with lib.maintainers; [NotAShelf];
    platforms = lib.platforms.all;
  };
})
