{
  description = "Barely customizable Wayland status bar for Hyprland compositor.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    ...
  }: let
    systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    forEachSystem = nixpkgs.lib.genAttrs systems;
  in {
    devShells = forEachSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
        rustPkgs = rust-overlay.packages.${system};
      in rec {
        default = ashell;
        ashell = pkgs.mkShell {
          buildInputs = with pkgs; [
            pkg-config

            (rustPkgs.rust.override {
              extensions = ["rust-src"];
            })
          ];
        };
      }
    );
  };
}
