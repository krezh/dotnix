{ config, ... }:
{
  hmModules.desktop.kopia = {
    enable = true;
    repository = {
      type = "filesystem";
      path = "/mnt/kopia";
      passwordFile = "${config.sops.secrets."kopia/password".path}";
    };
    backups = {
      downloads = {
        paths = [ "/home/krezh/Downloads" ];
        schedule = "daily";
        retentionPolicy = {
          keepDaily = 2;
        };
      };
      obsidian = {
        paths = [ "/home/krezh/Obsidian" ];
        schedule = "daily";
        retentionPolicy = {
          keepDaily = 2;
        };
      };
      wow = {
        paths = [
          "/home/krezh/Games/Faugus/battlenet/drive_c/Program Files (x86)/World of Warcraft/_retail_/Interface"
          "/home/krezh/Games/Faugus/battlenet/drive_c/Program Files (x86)/World of Warcraft/_retail_/WTF"
        ];
        schedule = "daily";
        retentionPolicy = {
          keepDaily = 2;
        };
      };
    };
  };
}
