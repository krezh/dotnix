{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.nixosModules.desktop.battery;
in
{
  options.nixosModules.desktop.battery = {
    enable = lib.mkEnableOption "battery";
  };

  config = lib.mkIf cfg.enable {

    services = {
      upower.enable = true;
      power-profiles-daemon.enable = true;
      auto-cpufreq = {
        enable = false;
        settings = { };
      };

      tlp = {
        enable = false;
        settings = {
          PLATFORM_PROFILE_ON_AC = "performance";
          PLATFORM_PROFILE_ON_BAT = "low-power";
        };
      };
    };

    environment = {
      systemPackages = with pkgs; [ upower-notify ];
    };

    systemd.services.upower-notify = {
      enable = true;

      unitConfig = {
        Description = "";
        Documentation = "";
      };
      serviceConfig = {
        ExecStart = "${pkgs.upower-notify}/bin/upower-notify";
        Restart = "on-failure";
      };
    };
  };
}
