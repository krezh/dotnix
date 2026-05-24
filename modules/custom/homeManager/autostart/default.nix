{
  flake.modules.homeManager.modules =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      cfg = config.autostart;

      entryType = lib.types.submodule {
        options = {
          exec = lib.mkOption {
            type = lib.types.str;
          };

          description = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
          };

          after = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [ ];
          };

          wants = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [ ];
          };

          delay = lib.mkOption {
            type = lib.types.ints.unsigned;
            default = 0;
          };
        };
      };

      systemdRun = "${pkgs.systemd}/bin/systemd-run";

      mkExecStart =
        name: entry:
        let
          properties =
            map (u: "--property=After=${u}") entry.after
            ++ map (u: "--property=Wants=${u}") entry.wants
            ++ lib.optional (entry.delay > 0) "--on-active=${toString entry.delay}s";
          args = lib.concatStringsSep " " (
            [
              "--user"
              "--no-block"
              "--collect"
              "--unit=${name}"
            ]
            ++ properties
          );
        in
        "-${systemdRun} ${args} -- ${entry.exec}";
    in
    {
      options.autostart = {
        enableXdgAutostart = lib.mkEnableOption "XDG autostart support via systemd-xdg-autostart-generator";

        apps = lib.mkOption {
          type = lib.types.attrsOf entryType;
          default = { };
        };
      };

      config = lib.mkMerge [
        (lib.mkIf cfg.enableXdgAutostart {
          systemd.user.targets.graphical-session = {
            Unit = {
              Wants = [ "xdg-desktop-autostart.target" ];
              Before = [ "xdg-desktop-autostart.target" ];
            };
          };
        })

        (lib.mkIf (cfg.apps != { }) {
          systemd.user.services.autostart = {
            Unit = {
              After = [ "graphical-session.target" ];
              PartOf = [ "graphical-session.target" ];
            };
            Service = {
              Type = "oneshot";
              RemainAfterExit = true;
              ExecStart = lib.mapAttrsToList mkExecStart cfg.apps;
            };
            Install.WantedBy = [ "graphical-session.target" ];
          };
        })
      ];
    };
}
