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
      copy-on-select = "clipboard";
      right-click-action = "paste";
      auto-update = "off";
      gtk-single-instance = true;
    };
  };
}
