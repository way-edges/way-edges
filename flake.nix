{
  description = "way-edges";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      # Systems supported
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      # Helper function to generate packages for each system
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

      # Function to get package for a system
      packageFor =
        system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs { inherit system overlays; };

          rustPlatform = pkgs.makeRustPlatform {
            cargo = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
            rustc = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
          };

          manifest = (pkgs.lib.importTOML ./crates/way-edges/Cargo.toml).package;
        in
        rustPlatform.buildRustPackage {
          pname = manifest.name;
          inherit (manifest) version;

          buildInputs = with pkgs; [
            libxkbcommon
            cairo
            libpulseaudio
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          src = pkgs.lib.cleanSource ./.;

          RUSTFLAGS = [
            "--cfg tokio_unstable"
            "--cfg tokio_uring"
          ];

          meta = {
            description = "Lightweight wayland client focusing on widgets hidden in your screen edge.";
            homepage = "https://github.com/way-edges/way-edges";
            platforms = nixpkgs.lib.platforms.linux;
            license = nixpkgs.lib.licenses.mit;
            mainProgram = "way-edges";
          };
        };

      # Function to build dev shell
      devShellFor =
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
        in
        pkgs.mkShell {
          buildInputs = with pkgs; [
            (rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
            rust-analyzer
            rustfmt
            clippy
            pkg-config
            libxkbcommon
            cairo
            libpulseaudio
          ];

          RUSTFLAGS = [
            "--cfg tokio_unstable"
            "--cfg tokio_uring"
          ];
        };
    in
    {
      # Generate per-system outputs
      packages = forAllSystems (system: {
        default = packageFor system;
        way-edges = packageFor system;
      });

      devShells = forAllSystems (system: {
        default = devShellFor system;
      });

      formatter = forAllSystems (system: nixpkgs.legacyPackages.${system}.nixfmt-tree);

      # Home manager module that doesn't depend on system-specific logic
      homeManagerModules.default =
        {
          lib,
          pkgs,
          config,
          ...
        }:
        let
          cfg = config.programs.way-edges;
        in
        with lib;
        {
          options.programs.way-edges = {
            enable = mkEnableOption "way-edges";

            package = mkOption {
              type = types.package;
              description = "The way-edges package to use.";
              default = self.packages.${pkgs.system}.way-edges;
            };

            settings = mkOption {
              type = types.attrs;
              default = { };
              description = "way-edges configuration.";
              example = literalExpression ''
                {
                  groups = [
                    {
                      name = "hyprland";
                      widgets = [
                        {
                          edge = "top";
                          position = "right";
                          layer = "overlay";
                          monitor = "HDMI-A-1";
                          widget = {
                            type = "workspace";
                            thickness = 25;
                            length = "25%";
                            hover_color = "#ffffff22";
                            active_increase = 0.2;
                            active_color = "#6B8EF0";
                            deactive_color = "#000";
                          };
                        }
                      ];
                    }
                  ];
                }
              '';
            };
          };

          config = mkIf cfg.enable {
            home.packages = [ cfg.package ];

            xdg.configFile."way-edges/config.jsonc" = {
              text = builtins.toJSON cfg.settings;
            };
          };
        };
    };
}
