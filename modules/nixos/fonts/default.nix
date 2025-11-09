{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.nixosModules.desktop.fonts;
in
{
  options.nixosModules.desktop.fonts = {
    enable = lib.mkEnableOption "fonts";
  };

  config = lib.mkIf cfg.enable {
    environment = {
      sessionVariables = {
        FREETYPE_PROPERTIES = "truetype:interpreter-version=40 cff:no-stem-darkening=0 autofitter:no-stem-darkening=0";
        CAIRO_ANTIALIAS = "subpixel";
        HB_NO_SHAPE_ACCELERATOR = "1";
        GDK_SCALE = "1";
        GDK_DPI_SCALE = "1";
      };
    };
    fonts = {
      fontconfig = {
        enable = true;
        antialias = true;
        cache32Bit = true;
        hinting = {
          enable = true;
          autohint = false;
          style = "slight";
        };
        subpixel = {
          rgba = "rgb";
          lcdfilter = "default";
        };
        useEmbeddedBitmaps = true;
        allowBitmaps = true;
        defaultFonts = {
          serif = [
            "Rubik"
            "Noto Color Emoji"
          ];
          sansSerif = [
            "Rubik"
            "Noto Color Emoji"
          ];
          monospace = [
            "JetBrainsMono Nerd Font"
            "Noto Color Emoji"
          ];
          emoji = [ "Noto Color Emoji" ];
        };
      };
      fontDir.enable = true;
      enableDefaultPackages = true;
      packages = with pkgs; [
        corefonts # Microsoft free fonts
        gyre-fonts
        dejavu_fonts
        cantarell-fonts
        google-fonts
        source-code-pro
        source-sans-pro
        source-serif-pro
        ubuntu-classic # Ubuntu fonts
        unifont # some international languages
        noto-fonts
        noto-fonts-cjk-sans
        noto-fonts-color-emoji
        liberation_ttf
        dina-font
        proggyfonts
        inter
        inter-nerdfont
        rubik
        nerd-fonts.jetbrains-mono
      ];
    };
  };
}
