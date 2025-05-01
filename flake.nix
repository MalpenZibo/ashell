{
  description = "A ready to go Wayland status bar for Hyprland";

  inputs = {
    crane.url = "https://flakehub.com/f/ipetkov/crane/0.20.3";
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.791944";
    flake-utils.url = "https://flakehub.com/f/numtide/flake-utils/0.1.102";
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
          # `nix build` and `nix run`
          x86_64-linux.default =
            if "${system}" == "x86_64-linux"
            then
              craneLib.buildPackage {
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
              }
            else
              pkgs.stdenv.mkDerivation {
                name = "empty-package";
                version = "0.1.0";

                # No src needed for an empty package
                src = ./.;

                # Skip phases that aren't needed
                dontUnpack = true;
                dontBuild = true;
                dontConfigure = true;

                # Just create an empty directory
                installPhase = ''
                  mkdir -p $out
                '';
              };
        };
        # `nix develop`
        devShells.default = pkgs.mkShell {
          inherit buildInputs ldLibraryPath;

          LD_LIBRARY_PATH = ldLibraryPath;
        };
      }
    );
}
