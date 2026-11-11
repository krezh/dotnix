{
  flake.modules.nixos.thor =
    {
      lib,
      config,
      ...
    }:
    let
      user = "krezh";
      browsers = [
        {
          key = "h";
          desc = "Helium";
          cmd = lib.getExe config.home-manager.users.${user}.programs.helium.finalPackage;
        }
        {
          key = "z";
          desc = "Zen";
          cmd = lib.getExe config.home-manager.users.${user}.programs.zen-browser.finalPackage;
        }
      ];
    in
    {
      home-manager.users.${user} = {
        homeModules.island.features.whichKey.menus.browser = browsers;
        homeModules.wlr-which-key = {
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
          menus.browser = browsers;
        };
      };
    };
}
