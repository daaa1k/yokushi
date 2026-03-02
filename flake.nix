{
  description = "yokushi — AI agent PreToolUse hook validator";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, crane }:
    let
      # Home Manager module — system-agnostic, exported at the top level.
      #
      # Usage in a Home Manager configuration:
      #
      #   inputs.yokushi.url = "github:daaa1k/yokushi";
      #
      #   { inputs, ... }: {
      #     imports = [ inputs.yokushi.homeManagerModules.default ];
      #     programs.yokushi = {
      #       enable = true;
      #       settings = {
      #         agents.claude-code.output = "json";
      #         rules = [
      #           { pattern = "git push"; message = "Use pull requests."; }
      #         ];
      #       };
      #     };
      #   }
      hmModule = { config, lib, pkgs, ... }:
        let
          cfg = config.programs.yokushi;
          yamlFormat = pkgs.formats.yaml { };
        in
        {
          options.programs.yokushi = {
            enable = lib.mkEnableOption "yokushi AI agent hook validator";

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.system}.default;
              defaultText = lib.literalExpression "yokushi.packages.\${pkgs.system}.default";
              description = "The yokushi package to install.";
            };

            settings = lib.mkOption {
              type = yamlFormat.type;
              default = { };
              description = ''
                Configuration for yokushi written to
                {file}`$XDG_CONFIG_HOME/yokushi/config.yaml`.

                Top-level keys mirror the YAML schema: `agents` and `rules`.
                See {file}`config.example.yaml` in the yokushi repository
                for a fully-annotated reference.
              '';
              example = lib.literalExpression ''
                {
                  agents = {
                    claude-code.output = "json";
                    default.output = "exit";
                  };
                  rules = [
                    { pattern = "git push"; message = "Direct git push is prohibited. Use pull requests."; }
                    { tool = "Write"; pattern = "\\.env$"; message = "Writing to .env files is prohibited."; }
                    { tool = "WebFetch"; pattern = "example\\.com"; message = "Access to example.com is restricted."; }
                  ];
                }
              '';
            };
          };

          config = lib.mkIf cfg.enable {
            home.packages = [ cfg.package ];

            xdg.configFile."yokushi/config.yaml" = lib.mkIf (cfg.settings != { }) {
              source = yamlFormat.generate "yokushi-config.yaml" cfg.settings;
            };
          };
        };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;

        # Arguments shared between dependency pre-build and the final build.
        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
        };

        # Pre-build only the dependencies to maximise cache reuse.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        yokushi = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });
      in
      {
        # --- packages ---------------------------------------------------
        packages = {
          default = yokushi;
          inherit yokushi;
        };

        # --- checks (run by `nix flake check`) --------------------------
        checks = {
          # Build the package itself.
          inherit yokushi;

          # Run clippy with --deny warnings.
          yokushi-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          # Run the test suite.
          yokushi-test = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        # --- devShell ---------------------------------------------------
        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = [
            pkgs.rust-analyzer
            pkgs.rustfmt
          ];
        };
      }
    ) // {
      # --- Home Manager module (system-agnostic) ----------------------
      homeManagerModules.default = hmModule;
    };
}
