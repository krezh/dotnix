{
  flake.modules.homeManager.desktop-utils =
    { pkgs, ... }:
    {
      home.packages = [ pkgs.valent ];
    };
  flake.modules.nixos.desktop-utils = {
    networking.firewall =
      let
        kde-connect = [
          {
            from = 1714;
            to = 1764;
          }
        ];
      in
      {
        allowedTCPPortRanges = kde-connect;
        allowedUDPPortRanges = kde-connect;
      };
  };
}
