{ config, lib, ... }:

with lib;

let
  cfg = config.nixosModules.desktop.smb-mount;
in
{
  options.nixosModules.desktop.smb-mount = {
    enable = mkEnableOption "SMB/CIFS mount service";

    server = mkOption {
      type = types.str;
      description = "SMB server address";
    };

    share = mkOption {
      type = types.str;
      description = "SMB share name";
    };

    mountPoint = mkOption {
      type = types.str;
      default = "/mnt/smb";
      description = "Local mount point";
    };

    credentialsFile = mkOption {
      type = types.path;
      default = "";
      description = "Path to the credentials file";
    };

    domain = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Domain for authentication (optional)";
    };

    securityMethod = mkOption {
      type = types.str;
      default = "ntlmssp";
      description = "Security method (ntlmssp, ntlm, krb5, etc.)";
    };
  };

  config = mkIf cfg.enable {
    boot.supportedFilesystems = [ "cifs" ];

    systemd.mounts = [
      {
        type = "cifs";
        what = "//${cfg.server}/${cfg.share}";
        where = cfg.mountPoint;
        mountConfig.Options = "credentials=${cfg.credentialsFile},uid=1000,gid=100,iocharset=utf8,file_mode=0644,dir_mode=0755,sec=${cfg.securityMethod},nobrl";
      }
    ];

    systemd.automounts = [
      {
        wantedBy = [ "multi-user.target" ];
        where = cfg.mountPoint;
        automountConfig.TimeoutIdleSec = "600";
      }
    ];
  };
}
