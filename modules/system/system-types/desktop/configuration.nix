{ inputs, ... }:
{
  flake.modules.nixos.system-desktop =
    { pkgs, ... }:
    {
      imports = with inputs.self.modules.nixos; [
        system-common
        fonts
        bluetooth
        pipewire
        xdg-settings
      ];

      environment.systemPackages = with pkgs; [
        ffmpegthumbnailer
        ffmpeg-headless
        libheif
        libheif.out
        nufraw
        nufraw-thumbnailer
        gdk-pixbuf
        usbutils
      ];

      programs.gdk-pixbuf.modulePackages = [ pkgs.librsvg ];
      environment.pathsToLink = [ "share/thumbnailers" ];

      services.earlyoom = {
        enable = true;
        freeMemThreshold = 5;
        enableNotifications = true;
        extraArgs = [
          "--avoid"
          "wine|\\.exe|gamescope"
        ];
      };
    };

  flake.modules.homeManager.system-desktop = {
    imports = with inputs.self.modules.homeManager; [
      system-common
      gtk-theme
      xdg-settings
    ];

    home.file.wallpapers = {
      recursive = true;
      source = ./wallpapers;
    };
  };
}
