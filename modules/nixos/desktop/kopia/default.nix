{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.nixosModules.desktop.kopia;
in
{
  options.nixosModules.desktop.kopia = {
    enable = lib.mkEnableOption "kopia backup service";

    user = lib.mkOption {
      type = lib.types.str;
      default = "kopia";
      description = "User to run Kopia backup service as";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "kopia";
      description = "Group to run Kopia backup service as";
    };

    repository = {
      type = lib.mkOption {
        type = lib.types.enum [
          "filesystem"
          "nfs"
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
        default = "/var/lib/kopia/repository";
        description = "Path to repository (for filesystem type), NFS server:path, or endpoint URL";
      };

      nfs = {
        server = lib.mkOption {
          type = lib.types.str;
          default = "";
          description = "NFS server address (extracted from path if not specified)";
        };

        mountPath = lib.mkOption {
          type = lib.types.str;
          default = "/mnt/kopia-nfs";
          description = "Local mount point for NFS share";
        };

        options = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [
            "rw"
            "hard"
            "intr"
            "rsize=8192"
            "wsize=8192"
            "timeo=14"
            "_netdev"
          ];
          description = "NFS mount options";
        };
      };

      passwordFile = lib.mkOption {
        type = lib.types.str;
        default = "/var/lib/kopia/repository.password";
        description = "Path to file containing repository password";
      };

      configFile = lib.mkOption {
        type = lib.types.str;
        default = "/var/lib/kopia/repository.config";
        description = "Path to Kopia repository configuration file";
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
                "pgzip"
                "lz4"
                "s2-default"
                "s2-better"
                "s2-parallel-4"
                "zstd-fastest"
                "zstd-default"
                "zstd-better"
                "zstd-best"
              ];
              default = "s2-default";
              description = "Compression algorithm to use";
            };

            retentionPolicy = {
              keepLatest = lib.mkOption {
                type = lib.types.nullOr lib.types.int;
                default = null;
                description = "Number of latest snapshots to keep";
              };

              keepHourly = lib.mkOption {
                type = lib.types.nullOr lib.types.int;
                default = null;
                description = "Number of hourly snapshots to keep";
              };

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

    ui = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable Kopia UI server";
      };

      port = lib.mkOption {
        type = lib.types.port;
        default = 51515;
        description = "Port for Kopia UI server";
      };

      bindAddress = lib.mkOption {
        type = lib.types.str;
        default = "127.0.0.1";
        description = "Bind address for Kopia UI server";
      };

      openFirewall = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Open firewall port for Kopia UI";
      };
    };

    extraConfig = lib.mkOption {
      type = lib.types.lines;
      default = "";
      description = "Extra configuration for Kopia";
    };
  };

  config = lib.mkIf cfg.enable {
    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.group;
      home = "/var/lib/kopia";
      createHome = true;
      description = "Kopia backup service user";
    };

    users.groups.${cfg.group} = { };

    environment.systemPackages = [ pkgs.kopia ];

    systemd.tmpfiles.rules = [
      "d /var/lib/kopia 0755 ${cfg.user} ${cfg.group} -"
      "d /var/log/kopia 0755 ${cfg.user} ${cfg.group} -"
    ]
    ++ lib.optionals (cfg.repository.type == "nfs") [
      "d ${cfg.repository.nfs.mountPath} 0755 ${cfg.user} ${cfg.group} -"
    ];

    # NFS mount configuration
    fileSystems = lib.mkIf (cfg.repository.type == "nfs") {
      ${cfg.repository.nfs.mountPath} = {
        device = cfg.repository.path;
        fsType = "nfs";
        options = cfg.repository.nfs.options;
      };
    };

    # All systemd services
    systemd.services = {
      # Repository initialization service
      kopia-repository-init = {
        description = "Initialize Kopia repository";
        wantedBy = [ "multi-user.target" ];
        before = lib.mapAttrsToList (name: _: "kopia-backup-${name}.service") cfg.backups;
        after = lib.optionals (cfg.repository.type == "nfs") [ "${cfg.repository.nfs.mountPath}.mount" ];
        requires = lib.optionals (cfg.repository.type == "nfs") [ "${cfg.repository.nfs.mountPath}.mount" ];
        serviceConfig = {
          Type = "oneshot";
          User = cfg.user;
          Group = cfg.group;
          RemainAfterExit = true;
          ExecStart =
            let
              initScript = pkgs.writeShellScript "kopia-init" ''
                set -euo pipefail

                export KOPIA_PASSWORD_FILE="${cfg.repository.passwordFile}"
                export KOPIA_CONFIG_PATH="${cfg.repository.configFile}"

                # Check if repository is already connected
                if ${pkgs.kopia}/bin/kopia repository status &>/dev/null; then
                  echo "Repository already connected"
                  exit 0
                fi

                # Initialize or connect to repository based on type
                case "${cfg.repository.type}" in
                  filesystem)
                    mkdir -p "${cfg.repository.path}"
                    if [ ! -f "${cfg.repository.path}/kopia.repository" ]; then
                      echo "Creating new filesystem repository at ${cfg.repository.path}"
                      ${pkgs.kopia}/bin/kopia repository create filesystem --path="${cfg.repository.path}"
                    else
                      echo "Connecting to existing filesystem repository at ${cfg.repository.path}"
                      ${pkgs.kopia}/bin/kopia repository connect filesystem --path="${cfg.repository.path}"
                    fi
                    ;;
                  nfs)
                    # For NFS, use the mounted path
                    REPO_PATH="${cfg.repository.nfs.mountPath}"
                    mkdir -p "$REPO_PATH"
                    if [ ! -f "$REPO_PATH/kopia.repository" ]; then
                      echo "Creating new NFS repository at $REPO_PATH (mounted from ${cfg.repository.path})"
                      ${pkgs.kopia}/bin/kopia repository create filesystem --path="$REPO_PATH"
                    else
                      echo "Connecting to existing NFS repository at $REPO_PATH (mounted from ${cfg.repository.path})"
                      ${pkgs.kopia}/bin/kopia repository connect filesystem --path="$REPO_PATH"
                    fi
                    ;;
                  s3|gcs|azure|sftp|webdav)
                    echo "Repository type ${cfg.repository.type} requires manual setup"
                    echo "Please run: kopia repository connect ${cfg.repository.type} --url=${cfg.repository.path}"
                    exit 1
                    ;;
                  *)
                    echo "Unknown repository type: ${cfg.repository.type}"
                    exit 1
                    ;;
                esac
              '';
            in
            "${initScript}";
        };
      };

      # Kopia UI service
      kopia-ui = lib.mkIf cfg.ui.enable {
        description = "Kopia UI Server";
        after = [
          "network.target"
          "kopia-repository-init.service"
        ];
        requires = [ "kopia-repository-init.service" ];
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          Type = "exec";
          User = cfg.user;
          Group = cfg.group;
          Restart = "always";
          RestartSec = "10s";
          ExecStart = "${pkgs.kopia}/bin/kopia server start --ui --address=${cfg.ui.bindAddress}:${toString cfg.ui.port}";
          Environment = [
            "KOPIA_PASSWORD_FILE=${cfg.repository.passwordFile}"
            "KOPIA_CONFIG_PATH=${cfg.repository.configFile}"
          ];
        };
      };

      # Maintenance service for repository optimization
      kopia-maintenance = {
        description = "Kopia repository maintenance";
        after = [ "kopia-repository-init.service" ];
        requires = [ "kopia-repository-init.service" ];
        serviceConfig = {
          Type = "oneshot";
          User = cfg.user;
          Group = cfg.group;
          ExecStart = "${pkgs.kopia}/bin/kopia maintenance run --full --safety=none";
          Environment = [
            "KOPIA_PASSWORD_FILE=${cfg.repository.passwordFile}"
            "KOPIA_CONFIG_PATH=${cfg.repository.configFile}"
          ];
        };
      };
    }
    // (lib.mapAttrs' (
      name: backup:
      lib.nameValuePair "kopia-backup-${name}" {
        description = "Kopia backup: ${name}";
        after = [ "kopia-repository-init.service" ];
        requires = [ "kopia-repository-init.service" ];
        serviceConfig = {
          Type = "oneshot";
          User = cfg.user;
          Group = cfg.group;
          ExecStart =
            let
              backupScript = pkgs.writeShellScript "kopia-backup-${name}" ''
                set -euo pipefail

                export KOPIA_PASSWORD_FILE="${cfg.repository.passwordFile}"
                export KOPIA_CONFIG_PATH="${cfg.repository.configFile}"

                # Set compression policy
                ${pkgs.kopia}/bin/kopia policy set --compression=${backup.compression} ${lib.concatStringsSep " " backup.paths}

                # Set retention policy
                ${lib.optionalString (backup.retentionPolicy.keepLatest != null)
                  "${pkgs.kopia}/bin/kopia policy set --keep-latest=${toString backup.retentionPolicy.keepLatest} ${lib.concatStringsSep " " backup.paths}"
                }
                ${lib.optionalString (backup.retentionPolicy.keepHourly != null)
                  "${pkgs.kopia}/bin/kopia policy set --keep-hourly=${toString backup.retentionPolicy.keepHourly} ${lib.concatStringsSep " " backup.paths}"
                }
                ${lib.optionalString (backup.retentionPolicy.keepDaily != null)
                  "${pkgs.kopia}/bin/kopia policy set --keep-daily=${toString backup.retentionPolicy.keepDaily} ${lib.concatStringsSep " " backup.paths}"
                }
                ${lib.optionalString (backup.retentionPolicy.keepWeekly != null)
                  "${pkgs.kopia}/bin/kopia policy set --keep-weekly=${toString backup.retentionPolicy.keepWeekly} ${lib.concatStringsSep " " backup.paths}"
                }
                ${lib.optionalString (backup.retentionPolicy.keepMonthly != null)
                  "${pkgs.kopia}/bin/kopia policy set --keep-monthly=${toString backup.retentionPolicy.keepMonthly} ${lib.concatStringsSep " " backup.paths}"
                }
                ${lib.optionalString (backup.retentionPolicy.keepAnnual != null)
                  "${pkgs.kopia}/bin/kopia policy set --keep-annual=${toString backup.retentionPolicy.keepAnnual} ${lib.concatStringsSep " " backup.paths}"
                }

                # Set exclude patterns
                ${lib.concatMapStringsSep "\n" (
                  pattern:
                  "${pkgs.kopia}/bin/kopia policy set --add-ignore='${pattern}' ${lib.concatStringsSep " " backup.paths}"
                ) backup.exclude}

                # Create snapshot
                echo "Creating snapshot for: ${lib.concatStringsSep ", " backup.paths}"
                ${pkgs.kopia}/bin/kopia snapshot create ${lib.concatStringsSep " " backup.paths} \
                  --description="Automated backup: ${name}" \
                  --tags=automated,${name}

                # Cleanup old snapshots
                echo "Cleaning up old snapshots..."
                ${pkgs.kopia}/bin/kopia maintenance run --full
              '';
            in
            "${backupScript}";
          StandardOutput = "journal";
          StandardError = "journal";
        };
      }
    ) cfg.backups);

    systemd.timers = {
      kopia-maintenance = {
        description = "Timer for Kopia repository maintenance";
        wantedBy = [ "timers.target" ];
        timerConfig = {
          OnCalendar = "weekly";
          Persistent = true;
          RandomizedDelaySec = "2h";
        };
      };
    }
    // (lib.mapAttrs' (
      name: backup:
      lib.nameValuePair "kopia-backup-${name}" {
        description = "Timer for Kopia backup: ${name}";
        wantedBy = [ "timers.target" ];
        timerConfig = {
          OnCalendar = backup.schedule;
          Persistent = true;
          RandomizedDelaySec = "1h";
        };
      }
    ) cfg.backups);

    # Firewall configuration
    networking.firewall.allowedTCPPorts = lib.mkIf (cfg.ui.enable && cfg.ui.openFirewall) [
      cfg.ui.port
    ];

    # Create password file if it doesn't exist
    system.activationScripts.kopia-password =
      lib.mkIf (cfg.repository.passwordFile == "/var/lib/kopia/repository.password")
        ''
          if [ ! -f "${cfg.repository.passwordFile}" ]; then
            echo "Creating Kopia repository password file..."
            ${pkgs.openssl}/bin/openssl rand -base64 32 > "${cfg.repository.passwordFile}"
            chown ${cfg.user}:${cfg.group} "${cfg.repository.passwordFile}"
            chmod 600 "${cfg.repository.passwordFile}"
            echo "Password file created at ${cfg.repository.passwordFile}"
            echo "IMPORTANT: Save this password in a secure location!"
          fi
        '';
  };
}
