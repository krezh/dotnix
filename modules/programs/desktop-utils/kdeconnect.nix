{
  flake.modules.homeManager.desktop-utils =
    { pkgs, ... }:
    {
      home.packages = [ pkgs.valent ];
      systemd.user.services.valent = {
        Unit = {
          Description = "Valent - KDE Connect implementation";
          After = [ "graphical-session.target" ];
          PartOf = [ "graphical-session.target" ];
        };
        Service = {
          ExecStart = "${pkgs.valent}/bin/valent --gapplication-service";
          Restart = "on-failure";
        };
        Install.WantedBy = [ "graphical-session.target" ];
      };
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
