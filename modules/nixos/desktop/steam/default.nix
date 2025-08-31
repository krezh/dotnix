{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.nixosModules.desktop.steam;
in
{
  options.nixosModules.desktop.steam = {
    enable = lib.mkEnableOption "steam";
  };

  config = lib.mkIf cfg.enable {
    programs.gamemode.enable = true;
    programs.gamemode.enableRenice = true;
    programs.gamemode.settings = {
      custom = {
        start = "${pkgs.libnotify}/bin/notify-send -t 5000 'GameMode' 'Started'";
        end = "${pkgs.libnotify}/bin/notify-send -t 5000 'GameMode' 'Ended'";
      };
    };
    environment = {
      systemPackages = with pkgs; [
        wine
        winetricks
        protontricks
        vulkan-tools
        sgdboop
        mangohud
        gamemode
        lsfg-vk
        lsfg-vk-ui
        lutris
        protonup
        heroic
        lact
      ];
      sessionVariables = {
        STEAM_EXTRA_COMPAT_TOOLS_PATHS = "$HOME/.steam/root/compatibilitytools.d";
      };
    };
    programs = {
      gamescope = {
        enable = true;
        capSysNice = true;
      };
      steam = {
        enable = true;
        remotePlay.openFirewall = true; # Open ports in the firewall for Steam Remote Play
        dedicatedServer.openFirewall = true; # Open ports in the firewall for Source Dedicated Server
        localNetworkGameTransfers.openFirewall = true; # Open ports in the firewall for Steam Local Network Game Transfers
        gamescopeSession.enable = true;
      };
    };
    systemd.user.services.steam = {
      enable = true;
      description = "Steam (no-GUI background startup)";
      wantedBy = [ "graphical-session.target" ];
      path = [ "/run/current-system/sw" ];
      serviceConfig = {
        ExecStart = "${lib.getExe pkgs.steam} -nochatui -nofriendsui -silent %U";
        Restart = "on-failure";
        RestartSec = "5s";
      };
    };
  };
}
