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
            buildInputs = [
              rust-bin.stable.latest.default
              rustPlatform.bindgenHook
              pkg-config
              libxkbcommon
              libGL
              pipewire
              libpulseaudio
              wayland
              vulkan-loader
            ];

            LD_LIBRARY_PATH = lib.makeLibraryPath [
              libpulseaudio
              wayland
              mesa.drivers
              vulkan-loader
              libGL
            ];
          };
        }
      );
}
