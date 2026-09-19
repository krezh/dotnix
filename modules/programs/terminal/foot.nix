{
  flake.modules.homeManager.terminal =
    { pkgs, config, ... }:
    {
      programs.foot = {
        enable = true;
        package = pkgs.foot;
        settings = {
          main = {
            font = "${config.var.fonts.mono}:size=${toString config.var.fonts.codeSize}";
            pad = "5x5 center";
            selection-target = "clipboard";
          };
          bell = {
            system = "no";
          };
          scrollback = {
            lines = 10000;
          };
          cursor = {
            style = "block";
            blink = "no";
          };
          mouse = {
            hide-when-typing = "yes";
          };
          mouse-bindings = {
            select-extend = "none";
            clipboard-paste = "BTN_RIGHT";
          };
        };
      };
    };
}
