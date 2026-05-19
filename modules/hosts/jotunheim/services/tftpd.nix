{
  flake.modules.nixos.jotunheim =
    { pkgs, ... }:
    {
      systemd.services.tftpd = {
        description = "TFTP Server (tftpd-hpa)";
        wantedBy = [ "multi-user.target" ];
        after = [ "network.target" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${pkgs.tftp-hpa}/bin/in.tftpd --listen --secure /mnt/tank/ipxe";
          Restart = "on-failure";
        };
      };

      networking.firewall.allowedUDPPorts = [ 69 ];
    };
}
