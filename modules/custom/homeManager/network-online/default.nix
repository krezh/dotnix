{
  flake.modules.homeManager.modules =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    {
      options.targets.network-online.enable = lib.mkEnableOption "user-level network-online.target";

      config = lib.mkIf config.targets.network-online.enable {
        systemd.user.targets.network-online = {
          Unit = {
            Description = "User Network is Online";
            Documentation = [ "man:systemd.special(7)" ];
          };
        };

        systemd.user.services.nm-wait-online = {
          Unit = {
            Description = "Network Manager Wait Online";
            Before = [ "network-online.target" ];
          };
          Service = {
            Type = "oneshot";
            ExecStart = "${pkgs.networkmanager}/bin/nm-online -s -q";
            RemainAfterExit = true;
            Environment = [ "NM_ONLINE_TIMEOUT=60" ];
          };
          Install.WantedBy = [ "network-online.target" ];
        };
      };
    };
}
