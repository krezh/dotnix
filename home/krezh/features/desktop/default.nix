{ config, ... }:
{
  imports = [
    ./hyprland
    ./gtk
    ./apps
    ./runners
    ./xdg
  ];

  # Import wallpapers into $HOME/wallpapers
  home.file."wallpapers" = {
    recursive = true;
    source = ./wallpapers;
  };

  hmModules.desktop.kopia = {
    enable = true;
    repository = {
      type = "filesystem";
      path = "/run/media/krezh/Ventoy/Backup";
      passwordFile = "${config.sops.secrets."kopia/password".path}";
    };
    backups = {
      downloads = {
        paths = [ "/home/krezh/Downloads" ];
        schedule = "daily";
        exclude = [
          "**/*.tmp"
          "**/*.log"
          "**/Trash/**"
        ];
        compression = "zstd";
        retentionPolicy = {
          keepDaily = 2;
        };
      };
      pictures = {
        paths = [ "/home/krezh/Pictures" ];
        schedule = "daily";
        exclude = [
          "**/*.tmp"
          "**/*.log"
          "**/Trash/**"
        ];
        compression = "zstd";
        retentionPolicy = {
          keepDaily = 2;
        };
      };
      wow = {
        paths = [
          "/home/krezh/.steam/steam/steamapps/compatdata/3300595845/pfx/drive_c/Program Files (x86)/World of Warcraft/_retail_/Interface"
          "/home/krezh/.steam/steam/steamapps/compatdata/3300595845/pfx/drive_c/Program Files (x86)/World of Warcraft/_retail_/WTF"
        ];
        schedule = "daily";
        exclude = [
          "**/*.tmp"
          "**/*.log"
          "**/Trash/**"
        ];
        compression = "zstd";
        retentionPolicy = {
          keepDaily = 2;
        };
      };
    };
  };

  services.udiskie = {
    enable = true;
  };
}
