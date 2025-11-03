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
        # Enable macOS-like rendering with interpreter version 40
        FREETYPE_PROPERTIES = "truetype:interpreter-version=40";
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
          # "slight" is closer to macOS, "none" is even closer but less sharp
          style = "slight";
        };
        subpixel = {
          rgba = "rgb";
          # "light" gives a macOS-like appearance, "default" is also good
          lcdfilter = "light";
        };
        useEmbeddedBitmaps = true;
        allowBitmaps = true;
        localConf = ''
          <?xml version="1.0"?>
          <!DOCTYPE fontconfig SYSTEM "fonts.dtd">
          <fontconfig>
            <!-- Enable LCD filter -->
            <match target="font">
              <edit name="lcdfilter" mode="assign">
                <const>lcdlight</const>
              </edit>
            </match>

            <!-- Disable hinting for fonts that look better without it -->
            <match target="font">
              <test name="family" compare="eq" ignore-blanks="true">
                <string>Inter</string>
              </test>
              <edit name="hintstyle" mode="assign">
                <const>hintslight</const>
              </edit>
            </match>

            <!-- Better emoji rendering -->
            <match target="pattern">
              <test name="family" compare="contains">
                <string>Emoji</string>
              </test>
              <edit name="hinting" mode="assign">
                <bool>false</bool>
              </edit>
              <edit name="antialias" mode="assign">
                <bool>true</bool>
              </edit>
            </match>

            <!-- Target macOS-like rendering for sans-serif fonts -->
            <match target="font">
              <test name="family" compare="contains">
                <string>Sans</string>
              </test>
              <edit name="rgba" mode="assign">
                <const>rgb</const>
              </edit>
              <edit name="lcdfilter" mode="assign">
                <const>lcdlight</const>
              </edit>
            </match>
          </fontconfig>
        '';
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
