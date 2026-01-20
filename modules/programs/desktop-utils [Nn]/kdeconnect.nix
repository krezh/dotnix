{
  flake.modules.homeManager.desktop-utils = {
    services.kdeconnect = {
      enable = true;
      indicator = true;
    };
  };
  flake.modules.nixos.desktop-utils = {
    networking.firewall = let
      kde-connect = [
        {
          from = 1714;
          to = 1764;
        }
      ];
    in {
      allowedTCPPortRanges = kde-connect;
      allowedUDPPortRanges = kde-connect;
    };
  };
}
