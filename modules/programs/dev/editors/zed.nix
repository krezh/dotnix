{ inputs, ... }:
{
  flake.modules.homeManager.editors =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    let
      alpha = lib.toLower (
        lib.fixedWidthString 2 "0" (lib.toHexString (builtins.floor (config.var.opacity * 255)))
      );
    in
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
          "xml"
          "catppuccin-icons"
          "editorconfig"
          "jj-lsp"
          "emmet"
        ];
        userSettings = {
          auto_update = false;
          agent_servers = {
            "claude-acp".type = "registry";
          };
          base_keymap = "VSCode";
          ui_font_size = 17;
          ui_font_family = config.var.fonts.sans;
          buffer_font_size = 15;
          buffer_font_family = config.var.fonts.mono;
          relative_line_numbers = "disabled";
          tab_size = 2;
          minimap.show = "never";
          edit_predictions.mode = "eager";
          agent = {
            enabled = true;
            tool_permissions.default = "allow";
            use_modifier_to_send = false;
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
              settings = rec {
                nixpkgs.expr = "import ${inputs.nixpkgs} { }";
                formatter.command = [ (lib.getExe pkgs.nixfmt) ];
                options = {
                  nixos.expr = ''
                    (let
                      pkgs = ${nixpkgs.expr};
                    in (pkgs.lib.evalModules {
                      modules = (import ${inputs.nixpkgs}/nixos/modules/module-list.nix) ++ [
                        ({...}: { nixpkgs.hostPlatform = "${pkgs.stdenv.hostPlatform.system}"; })
                      ];
                    })).options
                  '';
                  home-manager.expr = ''
                    (let
                      pkgs = ${nixpkgs.expr};
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
              command = [ (lib.getExe pkgs.nixfmt) ];
            };
            yaml-language-server.settings = {
              yaml.schemas = { };
            };
            just-lsp.settings = { };
            pyright.settings."python.analysis".typeCheckingMode = "off";
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
          theme_overrides."Catppuccin Mocha (blue)" = {
            "background" = "#1e1e2e00";
            "background.appearance" = "blurred";
            "editor.background" = "#1e1e2e${alpha}";
            "editor.gutter.background" = "#1e1e2e${alpha}";
            "editor.subheader.background" = "#181825${alpha}";
            "elevated_surface.background" = "#181825${alpha}";
            "panel.background" = "#181825${alpha}";
            "panel.overlay_background" = "#18182500";
            "scrollbar.track.background" = "#11111b00";
            "status_bar.background" = "#11111b${alpha}";
            "surface.background" = "#181825${alpha}";
            "tab.active_background" = "#1e1e2e00";
            "tab.inactive_background" = "#0b0b1100";
            "tab_bar.background" = "#11111b${alpha}";
            "terminal.ansi.background" = "#1e1e2e00";
            "terminal.background" = "#1e1e2e${alpha}";
            "title_bar.background" = "#11111b${alpha}";
            "title_bar.inactive_background" = "#171725${alpha}";
            "toolbar.background" = "#1e1e2e${alpha}";
          };
        };
      };
    };
}
