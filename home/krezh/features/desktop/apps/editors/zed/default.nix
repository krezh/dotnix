{ ... }:
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
    ];

    # This is the actual zed configuration
    userSettings = {
      auto_update = false;
      base_keymap = "VSCode";
      ui_font_size = 15;
      ui_font_family = "Inter";
      buffer_font_size = 15;
      buffer_font_family = "JetBrainsMono Nerd Font";
      relative_line_numbers = false;

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
          model = "GPT-4.1";
          provider = "copilot_chat";
        };

        inline_assistant_model = {
          provider = "anthropic";
          model = "claude-3-5-sonnet";
        };
      };

      # Configure languages
      languages = {
        "Nix" = {
          language_servers = [
            "nixd"
            "!nil"
          ];
        };
      };

      tabs = {
        file_icons = true;
        git_status = true;
      };

      # Configure LSPs
      lsp = {
        nixd = {
          initialization_options = {
            formatting = {
              command = [ "nixfmt" ];
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
      };

      # Disable telemetry
      telemetry = {
        diagnostics = false;
        metrics = false;
      };
    };
  };
}
