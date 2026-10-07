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
      cfg = config.nixosModules.swix;
      inherit (cfg) settings;
      toml = pkgs.formats.toml { };
    in
    {
      options.nixosModules.swix = {
        enable = mkEnableOption "socket-activated Swix activation and system-maintenance service";
        package = mkOption {
          type = types.package;
          default = pkgs.swix.override { nixPackage = config.nix.package; };
          description = "Swix package built against the configured system Nix implementation.";
        };
        user = mkOption {
          type = types.str;
          description = ''
            User allowed to request NixOS activation. This user is trusted as an
            administrator: the root service activates Nix store closures supplied
            through the user-owned socket.
          '';
        };
        settings = {
          appearance = {
            sansFont = mkOption {
              type = types.str;
              default = "sans-serif";
              description = "Font family used for interface text.";
            };
            monoFont = mkOption {
              type = types.str;
              default = "monospace";
              description = "Font family used for versions, metrics, and keycaps.";
            };
            symbolFont = mkOption {
              type = types.str;
              default = "Symbols Nerd Font";
              description = "Font family used for symbols, glyphs, and nerd icons.";
            };
            rounding = mkOption {
              type = types.ints.between 0 64;
              default = 15;
              description = "Outer corner radius in pixels.";
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
          disableKeybinds = mkOption {
            type = types.bool;
            default = false;
            description = "Whether keyboard shortcuts other than Escape are disabled.";
          };
        };
      };

      config = mkIf cfg.enable {
        environment = {
          systemPackages = [ cfg.package ];
          etc."swix/swix.toml".source = toml.generate "swix.toml" (
            {
              flake_dir = settings.flakeDir;
              nixos_flake = settings.nixosFlake;
              sans_font = settings.appearance.sansFont;
              mono_font = settings.appearance.monoFont;
              symbol_font = settings.appearance.symbolFont;
              rounding = settings.appearance.rounding;
              disable_keybinds = settings.disableKeybinds;
              keybinds = !settings.disableKeybinds;
            }
            // lib.optionalAttrs (settings.homeFlake != null) {
              home_flake = settings.homeFlake;
            }
          );
        };

        systemd = {
          tmpfiles.rules = [ "z /run/swix.sock 0600 ${cfg.user} root - -" ];

          sockets.swix = {
            description = "Swix privileged operation socket";
            wantedBy = [ "sockets.target" ];
            restartTriggers = [ (builtins.toJSON { inherit (cfg) user; }) ];
            socketConfig = {
              Accept = true;
              ListenStream = "/run/swix.sock";
              MaxConnections = 4;
              RemoveOnStop = true;
              SocketMode = "0600";
              SocketUser = cfg.user;
            };
          };

          services."swix@" = {
            description = "Swix privileged operation request";
            path = [
              config.nix.package
              pkgs.systemd
            ];
            restartIfChanged = false;
            stopIfChanged = false;
            unitConfig.X-StopOnRemoval = false;
            serviceConfig = {
              ExecStart = lib.getExe' cfg.package "swix-helper";
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
