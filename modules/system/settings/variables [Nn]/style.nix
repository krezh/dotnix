{
  flake.modules.generic.var = {lib, ...}: {
    options.var = lib.mkOption {
      type = lib.types.attrsOf lib.types.unspecified;
      default = {};
    };

    config.var = {
      fonts = {
        interfaceSize = 15;
        codeSize = 12;
        sans = "Rubik";
        mono = "JetBrainsMono Nerd Font";
        serif = "Rubik";
      };
      opacity = 0.98;
      borderSize = 3;
      rounding = 10;
    };
  };
}
