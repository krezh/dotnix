{
  flake.modules.nixos.jotunheim =

    {
      services.scrutiny = {
        enable = true;
        influxdb.enable = true;
        collector.enable = true;
      };

      services.nginx.virtualHosts."scrutiny.plexuz.xyz" = {
        enableACME = true;
        forceSSL = true;
        locations."/".proxyPass = "http://127.0.0.1:8080";
      };
    };
}
