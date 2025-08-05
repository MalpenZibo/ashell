{
  description = "A ready to go Wayland status bar for Hyprland";

  inputs = {
    crane.url = "github:ipetkov/crane";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      crane,
      nixpkgs,
      flake-parts,
      rust-overlay,
      ...
    }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } (
      { withSystem, ... }:
      {
        systems = [
          "x86_64-linux"
          "aarch64-linux"
          "x86_64-darwin"
          "aarch64-darwin"
        ];
        flake.modules.homeManager.ashell =
          {
            lib,
            pkgs,
            config,
            ...
          }:
          let
            cfg = config.services.ashell;
            tomlConfig = (pkgs.formats.toml { }).generate;
            settings = tomlConfig "ashell/config.toml" cfg.settings;
          in
          {
            options.services.ashell = {
              enable = lib.mkEnableOption "Enable ashell";
              package = lib.mkOption {
                type = lib.types.package;
                default = withSystem pkgs.stdenv.hostPlatform.system ({ config, ... }: config.packages.default);
              };
              settings = lib.mkOption {
                type = lib.types.attrs;
                default = { };
              };
            };

            config = lib.mkIf cfg.enable {
              systemd.user.services.ashell = {
                Install.WantedBy = [ "graphical-session.target" ];
                Unit = {
                  After = [ "graphical-session.target" ];
                  ConditionEnvironment = "WAYLAND_DISPLAY";
                  Description = "Ready to go Wayland status bar for Hyprland";
                  PartOf = [ "graphical-session.target" ];
                };
                Service = {
                  ExecStart = lib.getExe cfg.package;
                  X-Restart-Trigger = [ settings ];
                };
              };

              xdg.configFile."ashell/config.toml".source = settings;
            };
          };
        perSystem =
          { system, ... }:
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
              mesa
              vulkan-loader
              libGL
              libglvnd
            ];

            ldLibraryPath = pkgs.lib.makeLibraryPath runtimeDependencies;
          in
          {
            # `nix build` and `nix run`
            packages.default = craneLib.buildPackage {
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
          };
      }
    );
}
