{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.hmModules.desktop.kopia;

  # Script files
  initScript = pkgs.writeShellScript "kopia-init" (builtins.readFile ./scripts/init-repository.sh);
  backupScript = pkgs.writeShellScript "kopia-backup" (builtins.readFile ./scripts/backup.sh);
  maintenanceScript = pkgs.writeShellScript "kopia-maintenance" (
    builtins.readFile ./scripts/maintenance.sh
  );
  managerScript = pkgs.buildGoModule {
    pname = "kopia-manager";
    version = "1.0.0";
    src = ./kopia-manager;
    vendorHash = "sha256-Ki9MKnuqVf079sLud0f1+tvp40IoUsUxkH4862zGg4I=";

    buildInputs = with pkgs; [
      kopia
    ];

    postInstall = ''
      # Install shell completions
      installShellCompletion --cmd kopia-manager \
        --bash <($out/bin/kopia-manager completion bash) \
        --zsh <($out/bin/kopia-manager completion zsh) \
        --fish <($out/bin/kopia-manager completion fish)
    '';

    nativeBuildInputs = with pkgs; [
      installShellFiles
    ];

    meta = with lib; {
      description = "Kopia backup manager with CLI";
      license = licenses.mit;
      maintainers = [ ];
    };
  };

  # Configuration paths
  configFile = "${config.home.homeDirectory}/.config/kopia/repository.config";
  passwordFile = "${config.home.homeDirectory}/.config/kopia/repository.password";
  repositoryPath = "${config.home.homeDirectory}/.local/share/kopia-repository";
in
{
  options.hmModules.desktop.kopia = {
    enable = lib.mkEnableOption "kopia backup service";

    repository = {
      type = lib.mkOption {
        type = lib.types.enum [
          "filesystem"
          "s3"
          "gcs"
          "azure"
          "sftp"
          "webdav"
        ];
        default = "filesystem";
        description = "Repository type for Kopia backups";
      };

      path = lib.mkOption {
        type = lib.types.str;
        default = repositoryPath;
        description = "Path to repository (for filesystem type) or endpoint URL";
      };
    };

    backups = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.submodule {
          options = {
            paths = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              description = "List of paths to backup";
            };

            schedule = lib.mkOption {
              type = lib.types.str;
              default = "daily";
              description = "Backup schedule (systemd timer format)";
            };

            exclude = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ ];
              description = "List of patterns to exclude from backup";
            };

            compression = lib.mkOption {
              type = lib.types.enum [
                "inherit"
                "none"
                "deflate-best-compression"
                "deflate-best-speed"
                "deflate-default"
                "gzip"
                "gzip-best-compression"
                "gzip-best-speed"
                "lz4"
                "pgzip"
                "pgzip-best-compression"
                "pgzip-best-speed"
                "s2-better"
                "s2-default"
                "s2-parallel-4"
                "s2-parallel-8"
                "zstd"
                "zstd-best-compression"
                "zstd-better-compression"
                "zstd-fastest"
              ];
              default = "zstd";
              description = "Compression algorithm to use";
            };

            retentionPolicy = {
              keepDaily = lib.mkOption {
                type = lib.types.nullOr lib.types.int;
                default = 30;
                description = "Number of daily snapshots to keep";
              };

              keepWeekly = lib.mkOption {
                type = lib.types.nullOr lib.types.int;
                default = 12;
                description = "Number of weekly snapshots to keep";
              };

              keepMonthly = lib.mkOption {
                type = lib.types.nullOr lib.types.int;
                default = 12;
                description = "Number of monthly snapshots to keep";
              };

              keepAnnual = lib.mkOption {
                type = lib.types.nullOr lib.types.int;
                default = 3;
                description = "Number of annual snapshots to keep";
              };
            };
          };
        }
      );
      default = { };
      description = "Backup configurations";
    };

    gui = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Enable Kopia desktop GUI";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [
      pkgs.kopia
      pkgs.libnotify
      managerScript
    ]
    ++ lib.optionals cfg.gui.enable [
      pkgs.kopia-ui
    ];

    # Services
    systemd.user.services = {
      # Initialize repository service
      kopia-init = {
        Unit = {
          Description = "Initialize Kopia repository";
          After = [ "graphical-session.target" ];
        };
        Service = {
          Type = "oneshot";
          RemainAfterExit = true;
          ExecStart = "${initScript} ${cfg.repository.type} ${cfg.repository.path} ${configFile} ${passwordFile}";
        };
        Install.WantedBy = [ "default.target" ];
      };

      # Maintenance service
      kopia-maintenance = {
        Unit = {
          Description = "Kopia repository maintenance";
          After = [ "kopia-init.service" ];
          Requires = [ "kopia-init.service" ];
        };
        Service = {
          Type = "oneshot";
          ExecStart = "${maintenanceScript} ${configFile} ${passwordFile}";
        };
      };
    }
    // (lib.mapAttrs' (
      name: backup:
      lib.nameValuePair "kopia-backup-${name}" {
        Unit = {
          Description = "Kopia backup: ${name}";
          After = [ "kopia-init.service" ];
          Requires = [ "kopia-init.service" ];
        };
        Service = {
          Type = "oneshot";
          ExecStartPre = [
            "${pkgs.kopia}/bin/kopia policy set --compression=${backup.compression} --config-file='${configFile}' ${lib.concatStringsSep " " backup.paths}"
          ]
          ++ (lib.map (
            pattern:
            "${pkgs.kopia}/bin/kopia policy set --add-ignore='${pattern}' --config-file='${configFile}' ${lib.concatStringsSep " " backup.paths}"
          ) backup.exclude)
          ++ (
            lib.optionals (backup.retentionPolicy.keepDaily != null) [
              "${pkgs.kopia}/bin/kopia policy set --keep-daily=${toString backup.retentionPolicy.keepDaily} --config-file='${configFile}' ${lib.concatStringsSep " " backup.paths}"
            ]
            ++ lib.optionals (backup.retentionPolicy.keepWeekly != null) [
              "${pkgs.kopia}/bin/kopia policy set --keep-weekly=${toString backup.retentionPolicy.keepWeekly} --config-file='${configFile}' ${lib.concatStringsSep " " backup.paths}"
            ]
            ++ lib.optionals (backup.retentionPolicy.keepMonthly != null) [
              "${pkgs.kopia}/bin/kopia policy set --keep-monthly=${toString backup.retentionPolicy.keepMonthly} --config-file='${configFile}' ${lib.concatStringsSep " " backup.paths}"
            ]
            ++ lib.optionals (backup.retentionPolicy.keepAnnual != null) [
              "${pkgs.kopia}/bin/kopia policy set --keep-annual=${toString backup.retentionPolicy.keepAnnual} --config-file='${configFile}' ${lib.concatStringsSep " " backup.paths}"
            ]
          );
          ExecStart = "${backupScript} ${name} ${configFile} ${passwordFile} ${lib.concatStringsSep " " backup.paths}";
        };
      }
    ) cfg.backups);

    # Timers
    systemd.user.timers = {
      # Maintenance timer
      kopia-maintenance = {
        Unit = {
          Description = "Timer for Kopia repository maintenance";
        };
        Timer = {
          OnCalendar = "weekly";
          Persistent = true;
          RandomizedDelaySec = "2h";
        };
        Install.WantedBy = [ "timers.target" ];
      };
    }
    // (lib.mapAttrs' (
      name: backup:
      lib.nameValuePair "kopia-backup-${name}" {
        Unit = {
          Description = "Timer for Kopia backup: ${name}";
        };
        Timer = {
          OnCalendar = backup.schedule;
          Persistent = true;
          RandomizedDelaySec = "1h";
        };
        Install.WantedBy = [ "timers.target" ];
      }
    ) cfg.backups);
  };
}
