{
  flake.modules.nixos.protonvpn =
    { config, ... }:
    {
      sops.secrets."protonvpn/privateKey" = {
        owner = "root";
        mode = "0400";
      };

      sops.templates."protonvpn-nm-env" = {
        content = ''
          PROTONVPN_PRIVATE_KEY=${config.sops.placeholder."protonvpn/privateKey"}
        '';
      };

      boot.kernelModules = [ "wireguard" ];

      networking.networkmanager.ensureProfiles = {
        environmentFiles = [ config.sops.templates."protonvpn-nm-env".path ];
        profiles.proton = {
          connection = {
            id = "proton";
            type = "wireguard";
            interface-name = "Proton";
          };
          wireguard = {
            private-key = "$PROTONVPN_PRIVATE_KEY";
          };
          "wireguard-peer.288ekgckyNpFjvhuo6BqkABb1cDV89g2fpnU2+bwO38=" = {
            endpoint = "138.199.55.33:51820";
            allowed-ips = "0.0.0.0/0;::/0;";
            persistent-keepalive = 25;
          };
          ipv4 = {
            method = "manual";
            address1 = "10.2.0.2/32";
            dns = "10.2.0.1;";
          };
          ipv6 = {
            method = "manual";
            address1 = "2a07:b944::2:2/128";
            dns = "2a07:b944::2:1;";
          };
        };
      };
    };
}
