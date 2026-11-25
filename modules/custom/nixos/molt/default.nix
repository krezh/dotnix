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
      cfg = config.nixosModules.molt;
      inherit (cfg) settings;
      toml = pkgs.formats.toml { };
    in
    {
      options.nixosModules.molt = {
        enable = mkEnableOption "socket-activated Molt activation and system-maintenance service";
        package = lib.mkPackageOption pkgs "island" { };
        user = mkOption {
          type = types.str;
          description = ''
            User allowed to request NixOS activation. This user is trusted as an
            administrator: the root service activates Nix store closures supplied
            through the user-owned socket.
          '';
        };
        settings = {
          cleanup = {
            keepGenerations = mkOption {
              type = types.ints.between 0 100;
              default = 0;
              description = "Number of recent generations retained for each Nix profile during cleanup.";
            };
            journalMaxAge = mkOption {
              type = types.str;
              default = "0";
              description = "Archived journal retention used as the cleanup-screen default; 0 keeps none.";
            };
          };
          flakeDir = mkOption {
            type = types.str;
            description = "Directory containing the flake.";
          };
          homeFlake = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "Standalone Home Manager configuration name, or null to disable that target.";
          };
          nixosFlake = mkOption {
            type = types.str;
            description = "NixOS configuration name.";
          };
        };
      };

      config = mkIf cfg.enable {
        environment.etc."molt/molt.toml".source = toml.generate "molt.toml" (
          {
            flake_dir = settings.flakeDir;
            nixos_flake = settings.nixosFlake;
            cleanup = {
              keep_generations = settings.cleanup.keepGenerations;
              journal_max_age = settings.cleanup.journalMaxAge;
            };
          }
          // lib.optionalAttrs (settings.homeFlake != null) {
            home_flake = settings.homeFlake;
          }
        );

        systemd = {
          tmpfiles.rules = [ "z /run/molt.sock 0600 ${cfg.user} root - -" ];

          sockets.molt = {
            description = "Molt privileged operation socket";
            wantedBy = [ "sockets.target" ];
            restartTriggers = [ (builtins.toJSON { inherit (cfg) user; }) ];
            socketConfig = {
              Accept = true;
              ListenStream = "/run/molt.sock";
              MaxConnections = 4;
              RemoveOnStop = true;
              SocketMode = "0600";
              SocketUser = cfg.user;
            };
          };

          services."molt@" = {
            description = "Molt privileged operation request";
            path = [
              config.nix.package
              pkgs.systemd
            ];
            restartIfChanged = false;
            stopIfChanged = false;
            unitConfig.X-StopOnRemoval = false;
            serviceConfig = {
              ExecStart = lib.getExe' cfg.package "molt-helper";
              NoNewPrivileges = true;
              OOMPolicy = "stop";
              StandardInput = "socket";
              StandardOutput = "socket";
              StandardError = "journal";
              TimeoutStartSec = "37min";
              RuntimeMaxSec = "37min";
              UMask = "0077";
            };
          };
        };
      };
    };
}
