{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "nixon";
  version = "0.1.0";

  src = let
    fs = lib.fileset;
    s = ../.;
  in
    fs.toSource {
      root = s;
      fileset = fs.unions [
        (fs.fileFilter (file: builtins.any file.hasExt ["rs"]) (s + /src))
        (fs.fileFilter (file: builtins.any file.hasExt ["rs"]) (s + /tests))
        (fs.fileFilter (file: builtins.any file.hasExt ["rs"]) (s + /wasm))
        (s + /Cargo.lock)
        (s + /Cargo.toml)
        (s + /wasm/Cargo.toml)
        (s + /LICENSE)
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
