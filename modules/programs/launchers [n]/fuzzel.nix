{
  flake.modules.homeManager.launchers = {
    lib,
    pkgs,
    config,
    ...
  }: {
    catppuccin.fuzzel.enable = false;

    programs.fuzzel = {
      enable = true;
      settings = {
        main = {
          font = "${config.var.fonts.sans}:weight=bold:size=14";
          line-height = 28;
          fields = "name,generic,comment,categories,filename,keywords";
          terminal = lib.getExe pkgs.kitty;
          exit-on-keyboard-focus-loss = true;
          prompt = "'   '";
          icon-theme = "Papirus-Dark";
          layer = "overlay";
          horizontal-pad = 60;
          vertical-pad = 30;
          inner-pad = 10;
          image-size-ratio = 0.3;
          lines = 10;
          width = 35;
          letter-spacing = 0.5;
        };
        colors = {
          background = "1e1e2edd";
          text = "cdd6f4ff";
          prompt = "bac2deff";
          placeholder = "7f849cff";
          input = "cdd6f4ff";
          match = "f38ba8ff";
          selection = "b4befeff";
          selection-text = "f38ba8ff";
          selection-match = "f38ba8ff";
          counter = "7f849cff";
          border = "b4befeff";
        };
        border = {
          radius = config.var.rounding;
          width = config.var.borderSize;
        };
      };
    };
  };
}
