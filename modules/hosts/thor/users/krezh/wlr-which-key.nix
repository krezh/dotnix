{
  flake.modules.nixos.thor =
    {
      lib,
      config,
      pkgs,
      ...
    }:
    let
      user = "krezh";
    in
    {
      home-manager.users.${user} = {
        programs.wlr-which-key = {
          enable = true;
          settings = {
            font = "${config.var.fonts.mono} 14";
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
                cmd = lib.getExe config.home-manager.users.${user}.programs.helium.package;
              }
              {
                key = "z";
                desc = "Zen";
                cmd = lib.getExe config.home-manager.users.${user}.programs.zen-browser.package;
              }
              {
                key = "b";
                desc = "Brave";
                cmd = lib.getExe pkgs.brave;
              }
            ];
          };
        };
      };
    };
}
