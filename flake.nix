{
  description = "Barely customizable Wayland status bar for Hyprland compositor.";

  inputs = {
    naersk.url = "github:nmattia/naersk/master";
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
  outputs = { self, naersk, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          naersk-lib = pkgs.callPackage naersk { };
          pkgs = import nixpkgs {
            inherit system overlays;
          };
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
          # `nix build`
          defaultPackage = naersk-lib.buildPackage {
            pname = "ashell";
            src = ./.;
            doCheck = true;
            nativeBuildInputs = [ pkgs.makeWrapper ];
            # buildInputs = with pkgs; [
            #   xorg.libxcb
            # ];
            postInstall = ''
              wrapProgram "$out/bin/ashell" --prefix LD_LIBRARY_PATH : "${libPath}"
            '';
          };

          # `nix run`
          defaultApp = flake-utils.lib.mkApp {
            drv = self.defaultPackage."${system}";
          };

          # `nix develop`
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

            LD_LIBRARY_PATH = libPath;
          };
        }
      );
}
