{
  inputs,
  ...
}:
{
  flake.modules.homeManager.editors =
    {
      lib,
      pkgs,
      config,
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
          "json5"
          "log"
          "wakatime"
          "xml"
        ];
        userSettings = {
          auto_update = false;
          base_keymap = "VSCode";
          ui_font_size = 17;
          ui_font_family = "Rubik";
          buffer_font_size = 15;
          buffer_font_family = "${config.var.fonts.mono}";
          relative_line_numbers = "disabled";
          tab_size = 2;
          minimap.show = "never";
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
            use_modifier_to_send = false;
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
                nixpkgs.expr = "import ${inputs.nixpkgs} { }";
                options = {
                  nixos.expr = ''
                    (let
                      pkgs = import ${inputs.nixpkgs} { };
                    in (pkgs.lib.evalModules {
                      modules = (import ${inputs.nixpkgs}/nixos/modules/module-list.nix) ++ [
                        ({...}: { nixpkgs.hostPlatform = "${pkgs.stdenv.hostPlatform.system}"; })
                      ];
                    })).options
                  '';
                  home-manager.expr = ''
                    (let
                      pkgs = import ${inputs.nixpkgs} { };
                      lib = import ${inputs.home-manager}/modules/lib/stdlib-extended.nix pkgs.lib;
                    in (lib.evalModules {
                      modules = (import ${inputs.home-manager}/modules/modules.nix) {
                        inherit lib pkgs;
                        check = false;
                      };
                    })).options
                  '';
                };
              };
            };
            nil.settings.formatting = {
              command = [ "${lib.getExe pkgs.nixfmt-rfc-style}" ];
              args = [
                "-w"
                "300"
              ];
            };
            yaml-language-server.settings = {
              yaml.schemas = {
                "https://taskfile.dev/schema.json" = [
                  "Taskfile*.yml"
                  "Taskfile*.yaml"
                ];
              };
            };
            just-lsp.settings = { };
            pyright.settings."python.analysis".typeCheckingMode = "off";
            qml.binary.arguments = [ "-E" ];
          };
          inlay_hints = {
            enabled = true;
            show_type_hints = true;
            show_parameter_hints = true;
            show_other_hints = true;
            edit_debounce_ms = 700;
            scroll_debounce_ms = 50;
          };
          telemetry = {
            diagnostics = false;
            metrics = false;
          };
        };
      };
    };
}
