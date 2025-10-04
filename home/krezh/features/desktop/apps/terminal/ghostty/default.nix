{ pkgs, ... }:
{
  programs.ghostty = {
    enable = true;
    package = pkgs.ghostty;
    enableFishIntegration = true;
    installBatSyntax = true;
    settings = {
      font-family = "JetBrainsMono Nerd Font";
      font-size = 12;
      font-style = "Bold";
      font-thicken = true;
      copy-on-select = "clipboard";
      right-click-action = "paste";
      auto-update = "off";
      gtk-single-instance = true;
      custom-shader-animation = true;
      custom-shader = "${./shaders/cursor.glsl}";
    };
  };
}
