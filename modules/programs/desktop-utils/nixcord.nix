{ inputs, ... }:
{
  flake.modules.homeManager.desktop-utils =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    {
      imports = [ inputs.nixcord.homeModules.nixcord ];

      programs.nixcord = {
        enable = true;

        discord.enable = false;

        vesktop = {
          enable = true;
          settings = {
            discordBranch = "stable";
            appBadge = true;
            arRPC = true;
            enableSplashScreen = false;
            customTitleBar = false;
            disableMinSize = true;
            minimizeToTray = true;
            tray = true;
            clickTrayToShowHide = true;
            staticTitle = false;
            hardwareAcceleration = true;
            hardwareVideoAcceleration = true;
          };
        };

        config = {
          useQuickCss = true;
          frameless = true;
          transparent = true;
          disableMinSize = true;
          autoUpdate = false;
          autoUpdateNotification = false;
          notifyAboutUpdates = false;
          themeLinks = [
            "https://catppuccin.github.io/discord/dist/catppuccin-mocha-blue.theme.css"
          ];

          plugins = {
            gameActivityToggle.enable = true;
            noF1.enable = true;
            webRichPresence.enable = true;
            webScreenShareFixes.enable = true;
            replaceGoogleSearch = {
              enable = true;
              replacementEngine = "custom";
              customEngineName = "Kagi";
              customEngineUrl = "https://kagi.com/search?q=";
            };
          };
        };

        extraConfig = {
          winCtrlQ = false;
          winNativeTitleBar = false;
          eagerPatches = false;
        };
      };

      autostart.apps.vesktop = {
        exec = "${lib.getExe config.programs.nixcord.finalPackage.vesktop}";
        after = [
          "network-online.target"
          "arRPC.service"
        ];
        wants = [ "network-online.target" ];
        delay = 2;
      };

      services.arrpc = {
        enable = true;
        package = pkgs.rsrpc;
      };

      systemd.user.services.arRPC = {
        Unit = {
          After = [ "network-online.target" ];
          Wants = [ "network-online.target" ];
        };
        Service.RestartSec = "5";
      };
    };
}
