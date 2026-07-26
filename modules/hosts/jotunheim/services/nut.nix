{
  flake.modules.nixos.jotunheim =
    { config, ... }:
    {
      sops.secrets."nut/password" = {
        sopsFile = ../secrets.sops.yaml;
      };

      power.ups = {
        enable = true;
        mode = "netclient";

        upsmon.monitor."ups" = {
          system = "ups@rpi01.lan.plexuz.xyz";
          type = "secondary";
          user = "ups";
          passwordFile = config.sops.secrets."nut/password".path;
        };
      };
    };
}
