{
  inputs,
  lib,
  config,
  ...
}:
let
  cfg = config.programs.nixcord;

  applyPostPatch =
    pkg:
    pkg.overrideAttrs (o: {
      postPatch = lib.concatLines (
        lib.optional (cfg.userPlugins != { }) "mkdir -p src/userplugins"
        ++ lib.mapAttrsToList (
          name: path:
          "ln -s ${lib.escapeShellArg path} src/userplugins/${lib.escapeShellArg name} && ls src/userplugins"
        ) cfg.userPlugins
      );

      postInstall = (o.postInstall or "") + ''
        cp package.json $out
      '';
    });

  vencord = applyPostPatch cfg.discord.vencord.package;
in
{
  imports = [
    inputs.nixcord.homeModules.nixcord
  ];
  programs.nixcord = {
    enable = true;
    discord.enable = true;
    vesktop = {
      enable = false;
      settings = {
        discordBranch = "stable";
        staticTitle = false;
        enableSplashScreen = false;
        splashTheming = true;
        splashColor = "rgb(186, 194, 222)";
        splashBackground = "rgb(30, 30, 46)";
        arRPC = true;
        minimizeToTray = false;
      };
    };
    config = {
      frameless = true;
      themeLinks = [ "https://catppuccin.github.io/discord/dist/catppuccin-mocha.theme.css" ];
      plugins = {
        fakeNitro.enable = true;
        gameActivityToggle.enable = true;
        noF1.enable = true;
        webRichPresence.enable = true;
      };
    };
  };
  systemd.user.services.nixcord = {
    Install = {
      WantedBy = [
        (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
      ];
    };
    Unit = {
      Description = "A Custom Discord Client";
    };
    Service = {
      ExecStart = "${lib.getExe (
        cfg.discord.package.override {
          withVencord = cfg.discord.vencord.enable;
          withOpenASAR = cfg.discord.openASAR.enable;
          enableAutoscroll = cfg.discord.autoscroll.enable;
          branch = cfg.discord.branch;
          inherit vencord;
        }
      )}";
      Restart = "on-failure";
      RestartSec = 5;
    };
  };
}
