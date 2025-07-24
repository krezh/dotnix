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
    fonts = {
      fontconfig = {
        antialias = true;
        cache32Bit = true;
        hinting.enable = true;
        hinting.autohint = true;
        subpixel.rgba = "rgb";
      };
      fontDir.enable = true;
      enableDefaultPackages = true;
      packages = with pkgs; [
        corefonts # Microsoft free fonts
        dejavu_fonts
        fira
        fira-mono
        google-fonts
        source-code-pro
        source-sans-pro
        source-serif-pro
        ubuntu_font_family # Ubuntu fonts
        unifont # some international languages
        noto-fonts
        noto-fonts-cjk-sans
        noto-fonts-emoji
        liberation_ttf
        fira-code
        fira-code-symbols
        mplus-outline-fonts.githubRelease
        dina-font
        proggyfonts
        nerd-fonts.caskaydia-cove
        nerd-fonts.caskaydia-mono
        nerd-fonts.droid-sans-mono
        nerd-fonts.ubuntu
        nerd-fonts.ubuntu-mono
        nerd-fonts.ubuntu-sans
      ];
    };
  };
}
