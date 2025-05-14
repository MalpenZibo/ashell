{
  description = "A ready to go Wayland status bar for Hyprland";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.791944";
    systems.url = "github:nix-systems/x86_64-linux";
    crane.url = "github:ipetkov/crane";
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
            src = builtins.path {
              name = "source";
              path = ./.;
              filter = path: type: baseNameOf path != ".git";
            };

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
        devShells.default = let
          scripts = {
            dx = {
              exec = ''$EDITOR $REPO_ROOT/flake.nix'';
              description = "Edit flake.nix";
            };
          };

          scriptPackages =
            pkgs.lib.mapAttrs
            (name: script: pkgs.writeShellScriptBin name script.exec)
            scripts;
        in
          pkgs.mkShell {
            inherit buildInputs ldLibraryPath;
            shellHook = ''
              export REPO_ROOT=$(git rev-parse --show-toplevel)
              echo "Available commands:"
              ${pkgs.lib.concatStringsSep "\n" (
                pkgs.lib.mapAttrsToList (
                  name: script: ''echo "  ${name} - ${script.description}"''
                )
                scripts
              )}
            '';

            packages = with pkgs;
              [
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
              ]
              ++ builtins.attrValues scriptPackages;

            LD_LIBRARY_PATH = ldLibraryPath;
          };
      }
    );
}
