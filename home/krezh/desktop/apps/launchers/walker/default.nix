{
  inputs,
  config,
  pkgs,
  ...
}:
{
  imports = [ inputs.walker.homeManagerModules.default ];

  programs.walker = {
    enable = true;
    runAsService = true;
    config = {
      force_keyboard_focus = true;
      close_when_open = true;
      click_to_close = true;
      selection_wrap = true;
      global_argument_delimiter = "#";
      exact_search_prefix = "'";
      theme = "catppuccin";
      disable_mouse = false;
      debug = false;
      shell = {
        anchor_top = false;
        anchor_bottom = false;
        anchor_left = false;
        anchor_right = false;
      };
      placeholders = {
        default = {
          input = "Search";
          list = "No Results";
        };
      };
      keybinds = {
        close = [ "Escape" ];
        next = [ "Down" ];
        previous = [ "Up" ];
        quick_activate = [ ];

      };
      providers = {
        default = [
          "desktopapplications"
          "calc"
        ];
        empty = [ "desktopapplications" ];
        max_results = 50;
        sets = { };
        max_results_provider = { };
        prefixes = [
          {
            prefix = ";";
            provider = "providerlist";
          }
          {
            prefix = ".";
            provider = "symbols";
          }
          {
            prefix = "=";
            provider = "calc";
          }
        ];
        actions = {
          fallback = [
            {
              action = "menus:open";
              label = "open";
              after = "Nothing";
            }
            {
              action = "erase_history";
              label = "clear hist";
              bind = "ctrl h";
              after = "AsyncReload";
            }
          ];
          dmenu = [
            {
              action = "select";
              default = true;
              bind = "Return";
            }
          ];
          providerlist = [
            {
              action = "activate";
              default = true;
              bind = "Return";
              after = "ClearReload";
            }
          ];
          calc = [
            {
              action = "copy";
              default = true;
              bind = "Return";
            }
            {
              action = "delete";
              bind = "ctrl d";
              after = "AsyncReload";
            }
            {
              action = "save";
              bind = "ctrl s";
              after = "AsyncClearReload";
            }
          ];
          desktopapplications = [
            {
              action = "start";
              default = true;
              bind = "Return";
            }
          ];
          unicode = [
            {
              action = "run_cmd";
              label = "select";
              default = true;
              bind = "Return";
            }
          ];
          clipboard = [
            {
              action = "copy";
              default = true;
              bind = "Return";
            }
            {
              action = "remove";
              bind = "ctrl d";
              after = "ClearReload";
            }
            {
              action = "remove_all";
              global = true;
              label = "clear";
              bind = "ctrl shift d";
              after = "ClearReload";
            }
            {
              action = "toggle_images";
              global = true;
              label = "toggle images";
              bind = "ctrl i";
              after = "ClearReload";
            }
            {
              action = "edit";
              bind = "ctrl o";
            }
          ];
        };
      };
    };

    themes = {
      catppuccin = {
        style =
          builtins.readFile ./themes/catppuccin/colors.css + builtins.readFile ./themes/catppuccin/style.css;
        layouts = {
          layout = builtins.readFile ./themes/catppuccin/layout.xml;
        };
      };
    };
  };

  # Restart walker and elephant services after home-manager activation to detect new .desktop files
  home.activation.restartWalker = config.lib.dag.entryAfter [ "writeBoundary" ] ''
    $DRY_RUN_CMD ${pkgs.systemd}/bin/systemctl --user restart elephant.service walker.service 2>/dev/null || true
  '';
}
