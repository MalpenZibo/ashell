{
  description = "A ready to go Wayland status bar for Hyprland";

  inputs = {
    crane.url = "https://flakehub.com/f/ipetkov/crane/0.20.3.tar.gz";
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.791944.tar.gz";
    flake-utils.url = "https://flakehub.com/f/numtide/flake-utils/0.1.102.tar.gz";
    rust-overlay = {
      url = "https://flakehub.com/f/oxalica/rust-overlay/0.1.1771.tar.gz";
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
        # `nix build` and `nix run`
        packages."${system}".default = craneLib.buildPackage {
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
