{ pkgs, config, ... }:
{
  home.file.".config/hyprpanel/modules.json".source = ./modules.json;
  home.file.".config/hyprpanel/modules.scss".source = ./modules.scss;

  programs.hyprpanel = {
    enable = true;
    settings = {
      bar = {
        customModules.storage = {
          paths = [
            "/"
            "${config.home.homeDirectory}"
            "/mnt/secondary"
          ];
          round = true;
          pollingInterval = 60000;
        };
        battery = {
          label = false;
          hideLabelWhenFull = false;
        };
        clock = {
          format = "%a %d/%m  %H:%M";
          showIcon = true;
        };
        media.show_active_only = true;
        notifications = {
          show_total = true;
          hideCountWhenZero = true;
        };
        layouts = {
          "0" = {
            left = [
              "dashboard"
              "workspaces"
              "windowtitle"
            ];
            middle = [ "media" ];
            right = [
              "custom/steelseries"
              "custom/github"
              "hypridle"
              "storage"
              "custom/wallpaper"
              "volume"
              "network"
              "bluetooth"
              "systray"
              "clock"
              "notifications"
            ];
          };
          "1" = {
            left = [
              "dashboard"
              "workspaces"
              "windowtitle"
            ];
            middle = [ "media" ];
            right = [
              "volume"
              "clock"
              "notifications"
            ];
          };
          "2" = {
            left = [
              "dashboard"
              "workspaces"
              "windowtitle"
            ];
            middle = [ "media" ];
            right = [
              "volume"
              "clock"
              "notifications"
            ];
          };
        };
        autoHide = "never";
        border.location = "none";
        launcher.autoDetectIcon = true;
        workspaces = {
          show_icons = false;
          show_numbered = true;
          showWsIcons = false;
          showApplicationIcons = false;
        };
      };

      menus = {
        clock = {
          time = {
            military = true;
            hideSeconds = true;
          };
          weather.enabled = false;
        };
        media = {
          hideAuthor = false;
          displayTime = false;
          displayTimeTooltip = false;
        };
        power = {
          lowBatteryNotification = false;
          showLabel = true;
        };
      };

      theme = {
        bar = {
          scaling = 90;
          buttons = {
            enableBorders = false;
            clock.enableBorder = false;
            battery = {
              spacing = "0.5em";
              enableBorder = false;
            };
            workspaces.enableBorder = false;
          };
          floating = false;
          menus = {
            enableShadow = true;
            menu.notifications.height = "60em";
          };
          border.location = "none";
          transparent = true;
          enableShadow = false;
        };
        font.size = "1.0rem";
        font.name = "Inter Nerd Font Regular";
        notification.enableShadow = true;
        osd.enableShadow = true;
        matugen = false;
      };

      notifications = {
        autoDismiss = true;
        showActionsOnHover = true;
      };

      wallpaper.enable = true;
      scalingPriority = "both";
    };
  };

  home.packages = [
    pkgs.hyprpanel
  ];
}
