{ inputs, ... }:
{
  imports = [ inputs.caelestia-shell.homeManagerModules.default ];
  programs.caelestia = {
    enable = true;
    systemd = {
      enable = true; # if you prefer starting from your compositor;
      target = "graphical-session.target";
      environment = [ ];
    };
    settings = {
      appearance = {
        anim = {
          durations = {
            scale = 1;
          };
        };
        font = {
          family = {
            material = "Material Symbols Rounded";
            mono = "Inter Nerd Font";
            sans = "Inter ExtraBold";
          };
          size = {
            scale = 1;
          };
        };
        padding = {
          scale = 1;
        };
        rounding = {
          scale = 1;
        };
        spacing = {
          scale = 1;
        };
        transparency = {
          enabled = true;
          base = 0.85;
          layers = 0.4;
        };
      };
      general = {
        apps = {
          terminal = [ "kitty" ];
          audio = [ "pwvucontrol" ];
          playback = [ "mpv" ];
          explorer = [ "nautilus" ];
        };
        battery = {
          warnLevels = [
            {
              level = 20;
              title = "Low battery";
              message = "You might want to plug in a charger";
              icon = "battery_android_frame_2";
            }
            {
              level = 10;
              title = "Did you see the previous message?";
              message = "You should probably plug in a charger <b>now</b>";
              icon = "battery_android_frame_1";
            }
            {
              level = 5;
              title = "Critical battery level";
              message = "PLUG THE CHARGER RIGHT NOW!!";
              icon = "battery_android_alert";
              critical = true;
            }
          ];
          criticalLevel = 3;
        };
        idle = {
          enabled = false;
          lockBeforeSleep = true;
          inhibitWhenAudio = true;
          timeouts = [
            {
              timeout = 180;
              idleAction = "lock";
            }
            {
              timeout = 300;
              idleAction = "dpms off";
              returnAction = "dpms on";
            }
            {
              timeout = 600;
              idleAction = [
                "systemctl"
                "suspend-then-hibernate"
              ];
            }
          ];
        };
      };
      background = {
        desktopClock = {
          enabled = true;
        };
        enabled = false;
        visualiser = {
          enabled = true;
          autoHide = true;
          rounding = 1;
          spacing = 1;
        };
      };
      bar = {
        clock = {
          showIcon = false;
        };
        dragThreshold = 20;
        entries = [
          {
            id = "logo";
            enabled = false;
          }
          {
            id = "workspaces";
            enabled = true;
          }
          {
            id = "spacer";
            enabled = true;
          }
          {
            id = "activeWindow";
            enabled = true;
          }
          {
            id = "spacer";
            enabled = true;
          }
          {
            id = "tray";
            enabled = true;
          }
          {
            id = "clock";
            enabled = true;
          }
          {
            id = "statusIcons";
            enabled = true;
          }
          {
            id = "power";
            enabled = true;
          }
        ];
        persistent = true;
        scrollActions = {
          brightness = true;
          workspaces = true;
          volume = true;
        };
        showOnHover = true;
        status = {
          showAudio = true;
          showBattery = true;
          showBluetooth = true;
          showKbLayout = false;
          showMicrophone = false;
          showNetwork = true;
          showLockStatus = true;
        };
        tray = {
          background = false;
          compact = false;
          iconSubs = [ ];
          recolour = false;
        };
        workspaces = {
          activeIndicator = true;
          activeLabel = "󰮯";
          activeTrail = true;
          label = "  ";
          occupiedBg = false;
          occupiedLabel = "󰮯";
          perMonitorWorkspaces = true;
          showWindows = false;
          shown = 5;
          showWindowsOnSpecialWorkspaces = false;
        };
      };
      border = {
        rounding = 10;
        thickness = 2;
      };
      dashboard = {
        enabled = true;
        dragThreshold = 50;
        mediaUpdateInterval = 500;
        showOnHover = true;
      };
      launcher = {
        actionPrefix = ">";
        actions = [
          {
            name = "Calculator";
            icon = "calculate";
            description = "Do simple math equations (powered by Qalc)";
            command = [
              "autocomplete"
              "calc"
            ];
            enabled = true;
            dangerous = false;
          }
          {
            name = "Scheme";
            icon = "palette";
            description = "Change the current colour scheme";
            command = [
              "autocomplete"
              "scheme"
            ];
            enabled = true;
            dangerous = false;
          }
          {
            name = "Wallpaper";
            icon = "image";
            description = "Change the current wallpaper";
            command = [
              "autocomplete"
              "wallpaper"
            ];
            enabled = true;
            dangerous = false;
          }
          {
            name = "Variant";
            icon = "colors";
            description = "Change the current scheme variant";
            command = [
              "autocomplete"
              "variant"
            ];
            enabled = true;
            dangerous = false;
          }
          {
            name = "Transparency";
            icon = "opacity";
            description = "Change shell transparency";
            command = [
              "autocomplete"
              "transparency"
            ];
            enabled = false;
            dangerous = false;
          }
          {
            name = "Random";
            icon = "casino";
            description = "Switch to a random wallpaper";
            command = [
              "caelestia"
              "wallpaper"
              "-r"
            ];
            enabled = true;
            dangerous = false;
          }
          {
            name = "Light";
            icon = "light_mode";
            description = "Change the scheme to light mode";
            command = [
              "setMode"
              "light"
            ];
            enabled = true;
            dangerous = false;
          }
          {
            name = "Dark";
            icon = "dark_mode";
            description = "Change the scheme to dark mode";
            command = [
              "setMode"
              "dark"
            ];
            enabled = true;
            dangerous = false;
          }
          {
            name = "Shutdown";
            icon = "power_settings_new";
            description = "Shutdown the system";
            command = [
              "systemctl"
              "poweroff"
            ];
            enabled = true;
            dangerous = true;
          }
          {
            name = "Reboot";
            icon = "cached";
            description = "Reboot the system";
            command = [
              "systemctl"
              "reboot"
            ];
            enabled = true;
            dangerous = true;
          }
          {
            name = "Logout";
            icon = "exit_to_app";
            description = "Log out of the current session";
            command = [
              "loginctl"
              "terminate-user"
              ""
            ];
            enabled = true;
            dangerous = true;
          }
          {
            name = "Lock";
            icon = "lock";
            description = "Lock the current session";
            command = [
              "loginctl"
              "lock-session"
            ];
            enabled = true;
            dangerous = false;
          }
          {
            name = "Sleep";
            icon = "bedtime";
            description = "Suspend then hibernate";
            command = [
              "systemctl"
              "suspend-then-hibernate"
            ];
            enabled = true;
            dangerous = false;
          }
        ];
        dragThreshold = 50;
        vimKeybinds = false;
        enableDangerousActions = false;
        maxShown = 7;
        maxWallpapers = 9;
        specialPrefix = "@";
        useFuzzy = {
          apps = true;
          actions = false;
          schemes = false;
          variants = false;
          wallpapers = false;
        };
        showOnHover = false;
        hiddenApps = [ ];
      };
      lock = {
        recolourLogo = false;
      };
      notifs = {
        actionOnClick = true;
        clearThreshold = 0.3;
        defaultExpireTimeout = 5000;
        expandThreshold = 20;
        expire = true;
      };
      osd = {
        enabled = true;
        enableBrightness = true;
        enableMicrophone = false;
        hideDelay = 2000;
      };
      paths = {
        mediaGif = "";
        sessionGif = "";
        wallpaperDir = "~/Wallpapers";
      };
      services = {
        audioIncrement = 0.1;
        defaultPlayer = "Spotify";
        gpuType = "";
        playerAliases = [
          {
            from = "com.github.th_ch.youtube_music";
            to = "YT Music";
          }
        ];
        weatherLocation = "";
        useFahrenheit = false;
        useTwelveHourClock = false;
        smartScheme = true;
        visualiserBars = 45;
      };
      session = {
        dragThreshold = 30;
        enabled = true;
        vimKeybinds = false;
        commands = {
          logout = [
            "loginctl"
            "terminate-user"
            ""
          ];
          shutdown = [
            "systemctl"
            "poweroff"
          ];
          hibernate = [
            "systemctl"
            "hibernate"
          ];
          reboot = [
            "systemctl"
            "reboot"
          ];
        };
      };
      sidebar = {
        dragThreshold = 80;
        enabled = true;
      };
      utilities = {
        enabled = true;
        maxToasts = 4;
        toasts = {
          audioInputChanged = true;
          audioOutputChanged = true;
          capsLockChanged = true;
          chargingChanged = true;
          configLoaded = true;
          dndChanged = true;
          gameModeChanged = true;
          numLockChanged = true;
        };
      };
    };
    cli = {
      enable = true; # Also add caelestia-cli to path;
      settings = {
        theme.enableGtk = false;
      };
    };
  };
}
