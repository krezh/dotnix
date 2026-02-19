{
  flake.modules.nixos.modules =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    with lib;
    let
      cfg = config.nixosModules.mount;

      mountOptions =
        { name, ... }:
        {
          options = {
            enable = mkEnableOption "this mount";
            type = mkOption {
              type = types.enum [
                "smb"
                "nfs"
              ];
              description = "Mount type (SMB/CIFS or NFS)";
            };
            server = mkOption {
              type = types.str;
              description = "Server address (hostname or IP)";
            };
            share = mkOption {
              type = types.str;
              description = "Share/export name or path";
            };
            mountPoint = mkOption {
              type = types.str;
              default = "/mnt/${name}";
              description = "Local mount point";
            };
            credentialsFile = mkOption {
              type = types.nullOr types.path;
              default = null;
              description = "Path to the credentials file (SMB only)";
            };
            domain = mkOption {
              type = types.nullOr types.str;
              default = null;
              description = "Domain for authentication (SMB only, optional)";
            };
            securityMethod = mkOption {
              type = types.str;
              default = "ntlmssp";
              description = "Security method for SMB";
            };
            smbVersion = mkOption {
              type = types.nullOr types.str;
              default = "3.1.1";
              description = "SMB protocol version";
            };
            uid = mkOption {
              type = types.int;
              default = 1000;
              description = "User ID for mounted files";
            };
            gid = mkOption {
              type = types.int;
              default = 100;
              description = "Group ID for mounted files";
            };
            nfsVersion = mkOption {
              type = types.nullOr types.str;
              default = null;
              description = "NFS version - NFS only";
            };
            extraOptions = mkOption {
              type = types.listOf types.str;
              default = [ ];
              description = "Additional mount options";
            };
            timeoutIdleSec = mkOption {
              type = types.int;
              default = 600;
              description = "Idle timeout in seconds before unmounting";
            };
            autoMount = mkOption {
              type = types.bool;
              default = true;
              description = "Enable automounting";
            };
          };
        };

      enabledMounts = filterAttrs (_name: mount: mount.enable) cfg.mounts;

      mkSmbOptions =
        mount:
        let
          opts = [
            "uid=${toString mount.uid}"
            "gid=${toString mount.gid}"
            "sec=${mount.securityMethod}"
          ]
          ++ (optional (mount.smbVersion != null) "vers=${mount.smbVersion}")
          ++ (optional (mount.credentialsFile != null) "credentials=${mount.credentialsFile}")
          ++ (optional (mount.domain != null) "domain=${mount.domain}")
          ++ mount.extraOptions;
        in
        concatStringsSep "," opts;

      mkNfsOptions =
        mount:
        let
          opts =
            (optional (mount.nfsVersion != null) "vers=${mount.nfsVersion}")
            ++ [
              "rsize=8192"
              "wsize=8192"
              "hard"
              "timeo=10"
              "retrans=2"
              "noatime"
              "async"
            ]
            ++ mount.extraOptions;
        in
        concatStringsSep "," opts;

      mkSystemdMount = _name: mount: {
        type = if mount.type == "smb" then "cifs" else "nfs";
        what =
          if mount.type == "smb" then
            "//${mount.server}/${mount.share}"
          else
            "${mount.server}:${mount.share}";
        where = mount.mountPoint;
        mountConfig.Options = [
          (if mount.type == "smb" then mkSmbOptions mount else mkNfsOptions mount)
        ];
        unitConfig.Before = [ "remote-fs.target" ];
        after = [ "network-online.target" ];
        wants = [ "network-online.target" ];
      };

      mkSystemdAutoMount = _name: mount: {
        wantedBy = [ "remote-fs.target" ];
        where = mount.mountPoint;
        automountConfig.TimeoutIdleSec = toString mount.timeoutIdleSec;
        unitConfig.DefaultDependencies = false;
        after = [ "network-online.target" ];
        wants = [ "network-online.target" ];
      };
    in
    {
      options.nixosModules.mount = {
        enable = mkEnableOption "SMB/CIFS and NFS mount service";
        mounts = mkOption {
          type = types.attrsOf (types.submodule mountOptions);
          default = { };
          description = "SMB and NFS mounts configuration";
        };
      };

      config = mkIf cfg.enable {
        boot.supportedFilesystems =
          (optional (any (m: m.type == "smb") (attrValues enabledMounts)) "cifs")
          ++ (optional (any (m: m.type == "nfs") (attrValues enabledMounts)) "nfs");

        services.rpcbind.enable = mkIf (any (m: m.type == "nfs") (attrValues enabledMounts)) true;

        systemd = {
          tmpfiles.rules = map (
            mount: "d ${mount.mountPoint} 0755 ${toString mount.uid} ${toString mount.gid} -"
          ) (attrValues enabledMounts);
          mounts = mapAttrsToList mkSystemdMount enabledMounts;
          automounts = mapAttrsToList mkSystemdAutoMount (
            filterAttrs (_name: mount: mount.autoMount) enabledMounts
          );
        };

        environment.systemPackages = mkIf (any (m: m.type == "nfs") (attrValues enabledMounts)) [
          pkgs.nfs-utils
        ];

        systemd.services = mkIf (enabledMounts != { }) {
          "network-online.target".wantedBy = [ "multi-user.target" ];
        };
      };
    };
}
