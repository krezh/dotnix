{
  flake.modules.nixos.jotunheim =
    { config, ... }:
    {
      sops.secrets."nut/monitorPassword" = {
        sopsFile = ../secrets.sops.yaml;
      };

      power.ups = {
        enable = false;
        mode = "netclient";

        upsmon.monitor."ups" = {
          system = "<UPS_NAME>@<NUT_SERVER_HOST>";
          type = "secondary";
          user = "<NUT_MONITOR_USER>";
          passwordFile = config.sops.secrets."nut/monitorPassword".path;
        };
      };
    };
}
