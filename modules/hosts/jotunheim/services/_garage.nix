{
  flake.modules.nixos.jotunheim =
    { config, pkgs, ... }:
    {
      sops.secrets."garage/rpcSecret" = {
        sopsFile = ../secrets.sops.yaml;
      };

      sops.templates."garage-env" = {
        content = ''
          GARAGE_RPC_SECRET=${config.sops.placeholder."garage/rpcSecret"}
        '';
      };

      services.garage = {
        enable = true;
        package = pkgs.garage;
        environmentFile = config.sops.templates."garage-env".path;
        settings = {
          metadata_dir = "/var/lib/garage/meta";
          data_dir = "/mnt/tank/garage/data";
          db_engine = "lmdb";
          replication_factor = 1;
          rpc_bind_addr = "[::]:3901";
          rpc_secret = "\${GARAGE_RPC_SECRET}";
          s3_api = {
            s3_region = "garage";
            api_bind_addr = "[::]:3900";
          };
          admin = {
            api_bind_addr = "127.0.0.1:3903";
          };
        };
      };

      services.nginx.virtualHosts."s3.int.plexuz.xyz" = {
        enableACME = true;
        forceSSL = true;
        locations."/".proxyPass = "http://127.0.0.1:3900";
      };

      users.groups.garage = { };

      systemd.services.garage.serviceConfig.SupplementaryGroups = [ "garage" ];

      systemd.tmpfiles.rules = [
        "d /mnt/tank/garage/data 0770 root garage -"
      ];
    };
}
