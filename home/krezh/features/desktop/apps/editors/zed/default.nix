{
  hostname,
  lib,
  pkgs,
  inputs,
  ...
}:
{
  programs.zed-editor = {
    enable = true;
    extensions = [
      "nix"
      "git-firefly"
      "opentofu"
      "toml"
      "dockerfile"
      "jinja2"
      "just"
      "just-ls"
      "golangci-lint"
      "go-snippets"
      "scss"
      "basher"
      "qml"
    ];

    # This is the actual zed configuration
    userSettings = {
      auto_update = false;
      base_keymap = "VSCode";
      ui_font_size = 16;
      ui_font_family = "Inter";
      buffer_font_size = 14;
      buffer_font_family = "JetBrainsMono Nerd Font";
      relative_line_numbers = false;
      tab_size = 2;

      context_servers = {
        nixos = {
          source = "custom";
          enabled = true;
          command = "nix";
          args = [
            "run"
            "github:utensils/mcp-nixos"
            "--"
          ];
        };
      };

      # Enable copilot
      edit_predictions = {
        mode = "eager";
      };

      features = {
        edit_prediction_provider = "copilot";
      };

      # Configure the default assistant
      agent = {
        enabled = true;
        default_model = {
          provider = "copilot_chat";
          model = "gpt-4";
        };

        inline_assistant_model = {
          provider = "copilot_chat";
          model = "gpt-4";
        };
      };

      tabs = {
        file_icons = true;
        git_status = true;
      };

      # Configure languages
      languages = {
        "Nix" = {
          language_servers = [
            "nixd"
            "nil"
          ];
        };
      };

      file_types = {
        "OpenTofu" = [ "tf" ];
        "OpenTofu Vars" = [ "tfvars" ];
      };

      # Configure LSPs
      lsp = {
        nixd = {
          settings = {
            formatting = {
              command = [ "${lib.getExe pkgs.nixfmt-rfc-style}" ];
            };
            nixpkgs.expr = "import (builtins.getFlake \"${inputs.self}\").inputs.nixpkgs { }";
            options = rec {
              nixos.expr = "(builtins.getFlake \"${inputs.self}\").nixosConfigurations.${hostname}.options";
              home-manager.expr = "${nixos.expr}.home-manager.users.type.getSubOptions []";
            };
          };
        };
        nil = {
          settings = {
            formatting = {
              command = [ "${lib.getExe pkgs.nixfmt-rfc-style}" ];
            };
          };
        };
        yaml-language-server = {
          settings = {
            yaml = {
              schemas = {
                "https://taskfile.dev/schema.json" = [
                  "Taskfile*.yml"
                  "Taskfile*.yaml"
                ];
              };
            };
          };
        };
        just = { };
      };

      # Disable telemetry
      telemetry = {
        diagnostics = false;
        metrics = false;
      };
    };
  };
}
