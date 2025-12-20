{ pkgs, var, ... }:
{
  programs.ghostty = {
    enable = true;

    package = pkgs.ghostty;
    enableFishIntegration = true;
    installBatSyntax = true;
    settings = {
      font-family = "${var.fonts.mono}";
      font-size = 12;
      font-style = "Bold";
      font-thicken = true;
      copy-on-select = "clipboard";
      right-click-action = "paste";
      auto-update = "off";
      quit-after-last-window-closed = false;
      # gtk-single-instance = true;
      # custom-shader-animation = true;
      # custom-shader = "${./shaders/cursor.glsl}";
      shell-integration-features = true;
      app-notifications = "no-clipboard-copy";
      keybind = [
        "ctrl+g>r=reload_config"
        "ctrl+g>g=toggle_tab_overview"
        "ctrl+l=clear_screen"
      ];
      confirm-close-surface = "false";
    };
  };
}
