{
  description = "A ready to go Wayland status bar for Hyprland";

  inputs = {
    crane.url = "github:ipetkov/crane";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { crane, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          
          craneLib = crane.mkLib pkgs;

          buildInputs = with pkgs; [
            rust-bin.stable.latest.default
            rustPlatform.bindgenHook
            pkg-config
            libxkbcommon
            libGL
            pipewire
            libpulseaudio
            wayland
            vulkan-loader
            udev
          ];

          runtimeDependencies = with pkgs; [
            libpulseaudio
            wayland
            mesa.drivers
            vulkan-loader
            libGL
            libglvnd
          ];
            
          ldLibraryPath = pkgs.lib.makeLibraryPath runtimeDependencies;
        in
        {
          # `nix build` and `nix run`
          defaultPackage = craneLib.buildPackage {
            src = ./.;

            nativeBuildInputs = with pkgs; [
              makeWrapper
              pkg-config
              autoPatchelfHook # Add runtimeDependencies to rpath
            ];

            inherit buildInputs runtimeDependencies ldLibraryPath;

            postInstall = ''
              wrapProgram "$out/bin/ashell" --prefix LD_LIBRARY_PATH : "${ldLibraryPath}"
            '';
          };

          # `nix develop`
          devShells.default = pkgs.mkShell {
            inherit buildInputs ldLibraryPath;

            LD_LIBRARY_PATH = ldLibraryPath;
          };
        }
      );
}

