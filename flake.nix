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

  outputs = { self, crane, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          
          craneLib = crane.mkLib pkgs;

          deps = with pkgs; [
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

          libPath = with pkgs; lib.makeLibraryPath [
            libpulseaudio
            wayland
            mesa.drivers
            vulkan-loader
            libGL
          ];
        in
        with pkgs;
        {
          # `nix build` and `nix run`
          defaultPackage = craneLib.buildPackage {
            src = craneLib.cleanCargoSource ./.;

            nativeBuildInputs = [ pkgs.makeWrapper ];

            buildInputs = deps;

            postInstall = ''
              wrapProgram "$out/bin/ashell" --prefix LD_LIBRARY_PATH : "${libPath}"
            '';
          };

          # `nix develop`
          devShells.default = mkShell {
            buildInputs = deps;

            LD_LIBRARY_PATH = libPath;
          };
        }
      );
}

