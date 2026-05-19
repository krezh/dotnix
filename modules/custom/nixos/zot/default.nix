{
  flake.modules.nixos.modules =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      inherit (lib)
        mkEnableOption
        mkIf
        mkOption
        types
        ;
      cfg = config.services.zot;
    in
    {
      options.services.zot = {
        enable = mkEnableOption "Zot OCI container registry";
        settings = mkOption {
          type = types.attrs;
          default = { };
          description = "Zot configuration as a Nix attribute set, serialized to JSON.";
        };
      };

      config = mkIf cfg.enable {
        users = {
          users.zot = {
            isSystemUser = true;
            group = "zot";
          };
          groups.zot = { };
        };

        environment.etc."zot/config.json".text = builtins.toJSON cfg.settings;

        systemd.services.zot = {
          description = "Zot OCI Container Registry";
          wantedBy = [ "multi-user.target" ];
          after = [ "network.target" ];
          serviceConfig = {
            Type = "simple";
            User = "zot";
            Group = "zot";
            ExecStart = "${pkgs.zot}/bin/zot serve /etc/zot/config.json";
            Restart = "on-failure";
            RestartSec = "5s";
          };
        };
      };
    };
}
