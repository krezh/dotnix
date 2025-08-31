{ pkgs, config, ... }:
{
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
          pollingInterval = 10000;
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
              "hypridle"
              "storage"
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
            enableShadow = false;
            menu.notifications.height = "60em";
          };
          border.location = "none";
          transparent = true;
          enableShadow = false;
        };
        font.size = "1.0rem";
        notification.enableShadow = true;
        osd.enableShadow = false;
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

  home.packages = [ pkgs.hyprpanel ];
}
