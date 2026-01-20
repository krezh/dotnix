{inputs, ...}: {
  flake.modules.homeManager.launchers = {
    pkgs,
    config,
    ...
  }: {
    imports = [inputs.walker.homeManagerModules.default];

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
        hide_action_hints = true;
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
          close = ["Escape"];
          next = ["Down"];
          previous = ["Up"];
          quick_activate = [];
        };
        providers = {
          default = [
            "desktopapplications"
            "calc"
          ];
          empty = ["desktopapplications"];
          max_results = 50;
          sets = {};
          max_results_provider = {};
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
                action = "toggle_images";
                global = true;
                label = "toggle images";
                bind = "ctrl i";
                after = "ClearReload";
              }
            ];
          };
        };
      };

      themes = {
        catppuccin = {
          style = ''
            @define-color selected-text #8caaee;
            @define-color text #c6d0f5;
            @define-color base #24273a;
            @define-color border #8caaee;
            @define-color foreground #c6d0f5;
            @define-color background #24273a;

            * { all: unset; }
            * { font-family: "JetBrainsMono Nerd Font"; font-size: 20px; color: @text; }
            popover { background: @background; border: 3px @border; border-radius: 15px; padding: 10px; }
            scrollbar { opacity: 0; }
            .normal-icons { -gtk-icon-size: 16px; }
            .large-icons { -gtk-icon-size: 32px; }
            .box-wrapper { background: alpha(@base, 0.95); padding: 20px; border: 3px solid @border; border-radius: 15px; }
            .search-container { background: @base; padding: 10px; border-radius: 15px; }
            .input placeholder { opacity: 0.5; }
            .input:focus, .input:active { box-shadow: none; outline: none; }
            child:selected .item-box * { color: @selected-text; }
            .item-box { padding-left: 14px; }
            .item-text-box { all: unset; padding: 14px 0; }
            .item-subtext { font-size: 0px; min-height: 0px; margin: 0px; padding: 0px; }
            .item-image { margin-right: 14px; -gtk-icon-transform: scale(0.9); }
            .current { font-style: italic; }
            .keybind-hints { background: @background; padding: 10px; margin-top: 10px; }
          '';
        };
      };
    };

    # Restart walker and elephant services after home-manager activation
    home.activation.restartWalker = config.lib.dag.entryAfter ["writeBoundary"] ''
      $DRY_RUN_CMD ${pkgs.systemd}/bin/systemctl --user restart elephant.service walker.service 2>/dev/null || true
    '';
  };
}
