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
          font-family = "${config.var.fonts.mono}";
          font-size = 12;
          font-style = "Bold";
          font-thicken = true;
          copy-on-select = "clipboard";
          right-click-action = "paste";
          auto-update = "off";
          gtk-single-instance = true;
          quit-after-last-window-closed = false;
        };
      };
    };
}
