{ inputs, ... }:
{
  flake.modules.nixos.system-desktop =
    { pkgs, ... }:
    {
      imports = with inputs.self.modules.nixos; [
        system-base
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

      services.speechd.enable = false;
    };

  flake.modules.homeManager.system-desktop = {
    imports = with inputs.self.modules.homeManager; [
      system-base
      gtk-theme
      xdg-settings
    ];
  };
}
