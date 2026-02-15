let
  user = "krezh";
in
{
  flake.modules.nixos.thor =
    {
      pkgs,
      lib,
      config,
      ...
    }:
    {
      home-manager.users.${user} = {
        programs.wlr-which-key = {
          enable = true;
          settings = {
            font = "JetBrainsMono Nerd Font 14";
            background = "#1e1e2e";
            color = "#cdd6f4";
            border = "#89b4fa";
            separator = " ➜ ";
            border_width = 3;
            corner_r = 15;
          };
          menus = {
            browser = [
              {
                key = "h";
                desc = "Helium";
                cmd = lib.getExe pkgs.helium;
              }
              {
                key = "z";
                desc = "Zen";
                cmd = lib.getExe config.home-manager.users.${user}.programs.zen-browser.package;
              }
            ];
          };
        };
      };
    };
}
