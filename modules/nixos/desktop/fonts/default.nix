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
    # environment = {
    #   sessionVariables = {
    #     FREETYPE_PROPERTIES = "truetype:interpreter-version=38";
    #   };
    # };
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
          serif = [
            "Inter"
            "Noto Color Emoji"
          ];
          sansSerif = [
            "Inter"
            "Noto Color Emoji"
          ];
          monospace = [
            "Inter Nerd Font"
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
