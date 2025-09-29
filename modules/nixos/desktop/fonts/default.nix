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
        FREETYPE_PROPERTIES = "truetype:interpreter-version=35 cff:no-stem-darkening=0 autofitter:no-stem-darkening=0";
      };
    };
    fonts = {
      fontconfig = {
        antialias = true;
        cache32Bit = true;
        hinting.enable = true;
        hinting.autohint = false;
        hinting.style = "slight";
        subpixel = {
          rgba = "rgb";
          lcdfilter = "default";
        };
        useEmbeddedBitmaps = true;
        defaultFonts = {
          monospace = [ "Source Code Pro" ];
          sansSerif = [ "Source Sans Pro" ];
          serif = [ "Source Serif Pro" ];
        };
      };
      fontDir.enable = true;
      enableDefaultPackages = true;
      packages = with pkgs; [
        corefonts # Microsoft free fonts
        dejavu_fonts
        cantarell-fonts
        google-fonts
        fira
        fira-mono
        source-code-pro
        source-sans-pro
        source-serif-pro
        ubuntu_font_family # Ubuntu fonts
        unifont # some international languages
        noto-fonts
        noto-fonts-cjk-sans
        noto-fonts-emoji
        liberation_ttf
        dina-font
        proggyfonts
        inter
        inter-nerdfont
        nerd-fonts.caskaydia-cove
        nerd-fonts.caskaydia-mono
        nerd-fonts.ubuntu
        nerd-fonts.ubuntu-mono
        nerd-fonts.ubuntu-sans
        nerd-fonts.jetbrains-mono
        nerd-fonts.zed-mono
      ];
    };
  };
}
