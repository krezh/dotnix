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
      "github-actions"
    ];
    userSettings = {
      auto_update = false;
      base_keymap = "VSCode";
      ui_font_size = 17;
      ui_font_family = "Inter";
      buffer_font_size = 14;
      buffer_font_family = "JetBrainsMono Nerd Font";
      relative_line_numbers = false;
      tab_size = 2;
      minimap = {
        show = "always";
      };
      context_servers = {
        nixos = {
          enabled = true;
          source = "custom";
          command = "nix";
          args = [
            "run"
            "github:utensils/mcp-nixos"
            "--"
          ];
        };
      };
      edit_predictions.mode = "eager";
      agent = {
        enabled = true;
        always_allow_tool_actions = true;
        default_model = {
          provider = "copilot_chat";
          model = "gpt-4.1";
        };
      };
      tabs = {
        file_icons = true;
        git_status = true;
      };
      languages = {
        "Nix" = {
          language_servers = [
            "nixd"
            "nil"
          ];
        };
        "Just" = {
          tab_size = 2;
          hard_tabs = false;
        };
      };
      file_types = {
        "Just" = [
          "just"
          "justfile"
        ];
        "OpenTofu" = [ "tf" ];
        "OpenTofu Vars" = [ "tfvars" ];
      };
      lsp = {
        nixd = {
          settings = {
            formatting.command = [ "${lib.getExe pkgs.nixfmt-rfc-style}" ];
            nixpkgs.expr = "import (builtins.getFlake \"${inputs.self}\").inputs.nixpkgs { }";
            options = rec {
              nixos.expr = "(builtins.getFlake \"${inputs.self}\").nixosConfigurations.${hostname}.options";
              home-manager.expr = "${nixos.expr}.home-manager.users.type.getSubOptions []";
            };
          };
        };
        nil.settings.formatting = {
          command = [ "${lib.getExe pkgs.nixfmt-rfc-style}" ];
        };
        yaml-language-server.settings = {
          yaml = {
            schemas = {
              "https://taskfile.dev/schema.json" = [
                "Taskfile*.yml"
                "Taskfile*.yaml"
              ];
            };
          };
        };
        just-lsp.settings = { };
        pyright = {
          settings = {
            "python.analysis" = {
              typeCheckingMode = "off";
            };
          };
        };
      };

      inlay_hints = {
        enabled = true;
        show_type_hints = true;
        show_parameter_hints = true;
        show_other_hints = true;
        edit_debounce_ms = 700;
        scroll_debounce_ms = 50;
      };

      # Disable telemetry
      telemetry = {
        diagnostics = false;
        metrics = false;
      };
    };
  };
}
