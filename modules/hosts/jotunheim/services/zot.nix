{
  flake.modules.nixos.jotunheim =

    {
      services.zot = {
        enable = true;
        settings = {
          distSpecVersion = "1.1.0";
          storage.rootDirectory = "/mnt/tank/zot/data";
          http = {
            address = "127.0.0.1";
            port = 5000;
          };
          log.level = "info";
        };
      };

      services.nginx.virtualHosts."zot.plexuz.xyz" = {
        enableACME = true;
        forceSSL = true;
        locations."/".proxyPass = "http://127.0.0.1:5000";
      };

      systemd.tmpfiles.rules = [
        "d /mnt/tank/zot/data 0750 zot zot -"
      ];
    };
}
