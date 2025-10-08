{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.hmModules.desktop.kopia;
  kopiaManager = pkgs.buildGoModule {
    pname = "kopia-manager";
    version = "0.0.0";
    src = ./kopia-manager;
    vendorHash = "sha256-lxkd/FeeFstpr4LqMEWsECak27/i2k93faZTlXnT+jA=";
    buildInputs = [ pkgs.kopia ];
    postInstall = ''
      installShellCompletion --cmd kopia-manager \
        --bash <($out/bin/kopia-manager completion bash) \
        --zsh <($out/bin/kopia-manager completion zsh) \
        --fish <($out/bin/kopia-manager completion fish)
    '';
    nativeBuildInputs = with pkgs; [ installShellFiles ];
  };
  configFile = "${config.xdg.configHome}/kopia/repository.config";
  defaultPasswordFile = "${config.xdg.configHome}/kopia/repository.password";
  repositoryPath = "${config.xdg.dataHome}/kopia-repository";

  mkBackupService =
    name: backup:
    let
      configJson = mkBackupConfigJson name backup;
      passwordFile = cfg.repository.passwordFile;
    in
    {
      Unit = {
        Description = "Kopia backup: ${name}";
        After = [ "kopia-init.service" ];
        Requires = [ "kopia-init.service" ];
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${pkgs.writeShellScript "kopia-backup" (builtins.readFile ./scripts/backup.sh)} ${name} ${configFile} ${passwordFile} ${configJson}";
        Environment = [
          "KOPIA_CHECK_FOR_UPDATES=false"
        ];
      };
    };

  mkBackupConfigJson =
    name: backup:
    let
      json = builtins.toJSON {
        name = name;
        paths = backup.paths;
        exclude = backup.exclude;
        compression = backup.compression;
        retentionPolicy = backup.retentionPolicy;
      };
    in
    pkgs.writeTextFile {
      name = "kopia-backup-${name}-config.json";
      text = json;
    };

  mkBackupTimer = name: backup: {
    Unit = {
      Description = "Timer for Kopia backup: ${name}";
    };
    Timer = {
      OnCalendar = backup.schedule;
      Persistent = true;
      RandomizedDelaySec = backup.jitter;
    };
    Install.WantedBy = [ "timers.target" ];
  };
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

      passwordFile = lib.mkOption {
        type = lib.types.path;
        default = defaultPasswordFile;
        description = "Path to repository password file";
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
                "none"
                "gzip"
                "gzip-best-compression"
                "gzip-best-speed"
                "lz4"
                "zstd"
                "zstd-best-compression"
                "zstd-better-compression"
                "zstd-fastest"
              ];
              default = "zstd";
              description = "Compression algorithm to use";
            };

            jitter = lib.mkOption {
              type = lib.types.str;
              default = "15m";
              description = "Randomized delay to avoid all backups running at the same time";
            };

            retentionPolicy = {
              keepDaily = lib.mkOption {
                type = lib.types.int;
                default = 0;
                description = "Number of daily snapshots to keep";
              };

              keepWeekly = lib.mkOption {
                type = lib.types.int;
                default = 0;
                description = "Number of weekly snapshots to keep";
              };

              keepMonthly = lib.mkOption {
                type = lib.types.int;
                default = 0;
                description = "Number of monthly snapshots to keep";
              };

              keepAnnual = lib.mkOption {
                type = lib.types.int;
                default = 0;
                description = "Number of annual snapshots to keep";
              };

              keepHourly = lib.mkOption {
                type = lib.types.int;
                default = 0;
                description = "Number of hourly snapshots to keep";
              };

              keepLatest = lib.mkOption {
                type = lib.types.int;
                default = 0;
                description = "Number of latest snapshots to keep";
              };

              ignoreIdenticalSnapshots = lib.mkOption {
                type = lib.types.bool;
                default = false;
                description = "Ignore identical snapshots";
              };
            };
          };
        }
      );
      default = { };
      description = "Backup configurations";
    };

    maintenance = {
      schedule = lib.mkOption {
        type = lib.types.str;
        default = "weekly";
        description = "Maintenance schedule (systemd timer format)";
      };

      jitter = lib.mkOption {
        type = lib.types.str;
        default = "1h";
        description = "Randomized delay for maintenance to avoid resource conflicts";
      };
    };

    gui = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable Kopia desktop GUI";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [
      pkgs.kopia
      kopiaManager
    ]
    ++ lib.optionals cfg.gui.enable [ pkgs.kopia-ui ];

    systemd.user.services = {
      kopia-init = {
        Unit = {
          Description = "Initialize Kopia repository";
          After = [ "graphical-session.target" ];
        };
        Service = {
          Type = "oneshot";
          RemainAfterExit = true;
          ExecStart = "${pkgs.writeShellScript "kopia-init" (builtins.readFile ./scripts/init-repository.sh)} ${cfg.repository.type} ${cfg.repository.path} ${configFile} ${cfg.repository.passwordFile}";
          Environment = [
            "KOPIA_REPO_CONFIG=${cfg.repository.type}:${cfg.repository.path}"
            "KOPIA_CHECK_FOR_UPDATES=false"
          ];
        };
        Install.WantedBy = [ "default.target" ];
      };
      kopia-maintenance = {
        Unit = {
          Description = "Kopia repository maintenance";
          After = [ "kopia-init.service" ];
          Requires = [ "kopia-init.service" ];
        };
        Service = {
          Type = "oneshot";
          ExecStart = "${pkgs.writeShellScript "kopia-maintenance" (builtins.readFile ./scripts/maintenance.sh)} ${configFile} ${cfg.repository.passwordFile}";
          Environment = "KOPIA_CHECK_FOR_UPDATES=false";
        };
      };
    }
    // lib.mapAttrs' (
      name: backup: lib.nameValuePair "kopia-backup-${name}" (mkBackupService name backup)
    ) cfg.backups;

    systemd.user.timers = {
      kopia-maintenance = {
        Unit = {
          Description = "Timer for Kopia repository maintenance";
        };
        Timer = {
          OnCalendar = cfg.maintenance.schedule;
          Persistent = true;
          RandomizedDelaySec = cfg.maintenance.jitter;
        };
        Install.WantedBy = [ "timers.target" ];
      };
    }
    // lib.mapAttrs' (
      name: backup: lib.nameValuePair "kopia-backup-${name}" (mkBackupTimer name backup)
    ) cfg.backups;
  };
}
