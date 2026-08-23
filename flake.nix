{
  description = "Nixon, a compact lossless parser for the Nix language";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs?ref=nixos-unstable";

  outputs = {
    self,
    nixpkgs,
  }: let
    systems = ["x86_64-linux" "aarch64-linux"];
    forEachSystem = nixpkgs.lib.genAttrs systems;
    pkgsForEach = nixpkgs.legacyPackages;
  in {
    packages = forEachSystem (system: {
      nixon = pkgsForEach.${system}.callPackage ./nix/package.nix {};
      default = self.packages.${system}.nixon;
    });

    checks = forEachSystem (system: {
      package = self.packages.${system}.default;
    });

    devShells = forEachSystem (system: {
      default = pkgsForEach.${system}.callPackage ./nix/shell.nix {};
    });

    formatter = forEachSystem (system: pkgsForEach.${system}.alejandra); # soon...
    hydraJobs = self.checks;
  };
}
