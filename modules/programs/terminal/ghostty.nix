{
  flake.modules.homeManager.terminal =
    { pkgs, config, ... }:
    {
      programs.ghostty = {
        enable = true;
        package = pkgs.ghostty;
        enableFishIntegration = true;
        installBatSyntax = true;
        settings = {
          font-family = config.var.fonts.mono;
          font-size = config.var.fonts.codeSize;
          # font-style = "Bold";
          # font-thicken = true;
          copy-on-select = "clipboard";
          right-click-action = "paste";
          auto-update = "off";
          gtk-single-instance = true;
          quit-after-last-window-closed = false;
          selection-clear-on-copy = true;
          shell-integration-features = "no-cursor, sudo, title, ssh-env, ssh-terminfo, path";
          cursor-style = "block";
          # cursor_glide.glsl draws the cursor itself; hide ghostty's own.
          cursor-opacity = 0;
          custom-shader = "${./shaders/cursor_glide.glsl}";
          app-notifications = "no-clipboard-copy";
          resize-overlay = "never";
          window-padding-x = 5;
          window-padding-y = 5;
          confirm-close-surface = false;
        };
      };
    };
}
