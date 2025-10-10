{ config, ... }:
{
  imports = [
    ./hyprland
    ./gtk
    ./apps
    ./launchers
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
      wow = {
        paths = [
          "/home/krezh/.steam/steam/steamapps/compatdata/3300595845/pfx/drive_c/Program Files (x86)/World of Warcraft/_retail_/Interface"
          "/home/krezh/.steam/steam/steamapps/compatdata/3300595845/pfx/drive_c/Program Files (x86)/World of Warcraft/_retail_/WTF"
        ];
        schedule = "daily";
        retentionPolicy = {
          keepDaily = 2;
        };
      };
      wow-lutris = {
        paths = [
          "/home/krezh/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft/_retail_/Interface"
          "/home/krezh/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft/_retail_/WTF"
        ];
        schedule = "daily";
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
