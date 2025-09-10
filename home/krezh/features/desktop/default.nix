{ ... }:
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
    gui.enable = true;
    repository = {
      type = "filesystem";
      path = "/home/krezh/.local/share/kopia-repository";
    };
    backups = {
      downloads-backup = {
        paths = [ "/home/krezh/Downloads" ];
        schedule = "daily";
        exclude = [
          "**/*.tmp"
          "**/*.log"
          "**/Trash/**"
        ];
        compression = "zstd";
        retentionPolicy = {
          keepDaily = 7;
          keepWeekly = 4;
          keepMonthly = 3;
        };
      };
    };
  };
  services.udiskie = {
    enable = true;
  };
}
