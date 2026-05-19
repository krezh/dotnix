{
  flake.modules.nixos.jotunheim =

    {
      security.acme = {
        acceptTerms = true;
      };

      services.nginx.enable = true;

      networking.firewall.allowedTCPPorts = [
        80
        443
      ];
    };
}
