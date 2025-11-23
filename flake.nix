{
  description = "A ready to go Wayland status bar for Hyprland";

  inputs = {
    crane.url = "github:ipetkov/crane";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs =
    {
      crane,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      forAllSystems = with nixpkgs; (lib.genAttrs lib.systems.flakeExposed);
      perSystem = forAllSystems (system: rec {
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        craneLib = crane.mkLib pkgs;
        rustToolchain = pkgs.rust-bin.stable.latest;
        buildInputs = with pkgs; [
          rustToolchain.default
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
          mesa
          vulkan-loader
          libGL
          libglvnd
        ];
        ldLibraryPath = pkgs.lib.makeLibraryPath runtimeDependencies;
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
          meta.mainProgram = "ashell";
        };

        devShell = pkgs.mkShell {
          inherit ldLibraryPath;
          buildInputs = buildInputs ++ [
            pkgs.rust-analyzer-unwrapped
            pkgs.nixfmt-rfc-style
          ];

          RUST_SRC_PATH = "${rustToolchain.rust-src}/lib/rustlib/src/rust/library";
          LD_LIBRARY_PATH = ldLibraryPath;
        };

        formatter = pkgs.nixfmt-rfc-style;
      });
    in
    {
      packages = forAllSystems (system: {
        default = perSystem.${system}.defaultPackage;
      });
      devShells = forAllSystems (system: {
        default = perSystem.${system}.devShell;
      });
      formatter = forAllSystems (system: perSystem.${system}.formatter);
    };
}
