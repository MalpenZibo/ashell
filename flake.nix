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
        packages."${system}".lint = pkgs.writeShellApplication {
          name = "lint";
          runtimeInputs = with pkgs; [
            nixpkgs-fmt
            alejandra
            shellcheck
            statix
            rustfmt
            clippy
            cargo-udeps
            cargo-deny
            cargo-watch
          ];

          text = ''
            nixpkgs-fmt --check .
            alejandra --check .
            shellcheck --check-sourced --severity=warning --external-sources --source-path=. flake.nix
            statix check
            cargo fmt --all -- --check
            cargo clippy --all-targets --all-features -- -D warnings
            cargo udeps
            cargo deny check
            cargo watch -x 'test --all-features'
          '';
        };
        # `nix build` and `nix run`
        packages.x86_64-linux.default = craneLib.buildPackage {
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
