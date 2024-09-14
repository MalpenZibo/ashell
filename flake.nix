{
  description = "A ready to go Wayland status bar for Hyprland";

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
          # Fetch all the git dependencies
          clipboard_macos_src = builtins.fetchGit {
            url = "https://github.com/pop-os/window_clipboard.git";
            rev = "8a816d8f218e290041bb5ef6d3b695c38e0a53b7";
          };

          clipboard_wayland_src = builtins.fetchGit {
            url = "https://github.com/pop-os/window_clipboard.git";
            rev = "8a816d8f218e290041bb5ef6d3b695c38e0a53b7";
          };

          clipboard_x11_src = builtins.fetchGit {
            url = "https://github.com/pop-os/window_clipboard.git";
            rev = "8a816d8f218e290041bb5ef6d3b695c38e0a53b7";
          };

          dnd_src = builtins.fetchGit {
            url = "https://github.com/pop-os/window_clipboard.git";
            rev = "8a816d8f218e290041bb5ef6d3b695c38e0a53b7";
          };

          hyprland_src = builtins.fetchGit {
            url = "https://github.com/hyprland-community/hyprland-rs";
            rev = "3c8304e482a14d251518dd2ef1533538bf68d884";
          };

          hyprland_macros_src = builtins.fetchGit {
            url = "https://github.com/hyprland-community/hyprland-rs";
            rev = "3c8304e482a14d251518dd2ef1533538bf68d884";
          };

          iced_sctk_src = builtins.fetchGit {
            url = "https://github.com/MalpenZibo/iced_sctk";
            rev = "25ba868f710b83678f20ecae5f77109c73bc6c7d";
          };

          smithay_clipboard_src = builtins.fetchGit {
            url = "https://github.com/pop-os/smithay-clipboard";
            rev = "ab422ddcc95a9a1717df094f9c8fe69e2fdb2a27";
          };
          manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
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
          # `nix build`
          defaultPackage = rustPlatform.buildRustPackage rec {
            pname = manifest.name;
            version = manifest.version;
            cargoDeps = importCargoLock {
              lockFile = ./Cargo.lock;
              # Add all git dependencies' sources here
              sources = {
                "clipboard_macos" = clipboard_macos_src;
                "clipboard_wayland" = clipboard_wayland_src;
                "clipboard_x11" = clipboard_x11_src;
                "dnd" = dnd_src;
                "hyprland" = hyprland_src;
                "hyprland-macros" = hyprland_macros_src;
                "iced_sctk" = iced_sctk_src;
                "smithay-clipboard" = smithay_clipboard_src;
              };
              outputHashes = {
                "clipboard_macos-0.1.0" = lib.fakeHash;
                "clipboard_wayland-0.2.2" = lib.fakeHash;
                "clipboard_x11-0.4.2" = lib.fakeHash;
                "dnd-0.1.0" = lib.fakeHash;
                "hyprland-0.4.0-beta.1" = lib.fakeHash;
                "hyprland-macros-0.4.0-beta.1" = lib.fakeHash;
                "iced_sctk-0.1.0" = lib.fakeHash;
                "smithay-clipboard-0.8.0" = lib.fakeHash;
              };
            };

            src = ./.;

            cargoHash = lib.fakeHash;

            nativeBuildInputs = [ pkgs.makeWrapper ];

            buildInputs = deps;

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
            buildInputs = deps;

            LD_LIBRARY_PATH = libPath;
          };
        }
      );
}
