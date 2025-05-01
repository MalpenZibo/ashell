{
  description = "A ready to go Wayland status bar for Hyprland";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.791944";
    systems.url = "github:nix-systems/x86_64-linux";
    crane.url = "https://flakehub.com/f/ipetkov/crane/0.20.3";
    flake-utils.url = "https://flakehub.com/f/numtide/flake-utils/0.1.102";
    flake-utils.inputs.systems.follows = "systems";
    rust-overlay = {
      url = "https://flakehub.com/f/oxalica/rust-overlay/0.1.1771";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = {
    crane,
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem
    (
      system: let
        overlays = [(import rust-overlay)];
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
          mesa
          vulkan-loader
          libGL
          libglvnd
        ];

        ldLibraryPath = pkgs.lib.makeLibraryPath runtimeDependencies;
      in {
        packages = {
          default = craneLib.buildPackage {
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
        };
        devShells.default = pkgs.mkShell {
          inherit buildInputs ldLibraryPath;
          packages = with pkgs; [
            rust-bin.stable.latest.default
            pkg-config
            libxkbcommon
            libGL
            pipewire
            libpulseaudio
            wayland
            vulkan-loader
            udev
            autoPatchelfHook
            cargo-edit
            cargo-watch
            rust-analyzer
            nixpkgs-fmt
            rustfmt
            clippy
          ];

          LD_LIBRARY_PATH = ldLibraryPath;
        };
      }
    );
}
