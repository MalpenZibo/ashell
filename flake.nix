{
  description = "Barely customizable Wayland status bar for Hyprland compositor.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
        in
        with pkgs;
        {
          devShells.default = mkShell {
            buildInputs = [ rust-bin.stable.latest.default ];
          };
        }
      );
}
#
# {
#   description = "Barely customizable Wayland status bar for Hyprland compositor.";
#
#   inputs = {
#     nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
#     rust-overlay.url = "github:oxalica/rust-overlay";
#   };
#
#   outputs = {
#     nixpkgs,
#     rust-overlay,
#     ...
#   }: let
#     systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
#     forEachSystem = nixpkgs.lib.genAttrs systems;
#   in {
#     devShells = forEachSystem (
#       system: let
#         pkgs = nixpkgs.legacyPackages.${system};
#         rustPkgs = rust-overlay.packages.${system};
#       in rec {
#         default = ashell;
#         ashell = pkgs.mkShell {
#           buildInputs = with pkgs; [
#             pkg-config
#
#             (rustPkgs.rust.override {
#               extensions = ["rust-src"];
#             })
#           ];
#         };
#       }
#     );
#   };
# }
