{
  flake.modules.generic.var =
    { lib, ... }:
    {
      options.var = {
        opacity = lib.mkOption { type = lib.types.float; };
        borderSize = lib.mkOption { type = lib.types.int; };
        rounding = lib.mkOption { type = lib.types.int; };
        fonts = {
          sans = lib.mkOption { type = lib.types.str; };
          mono = lib.mkOption { type = lib.types.str; };
          serif = lib.mkOption { type = lib.types.str; };
          interfaceSize = lib.mkOption { type = lib.types.int; };
          codeSize = lib.mkOption { type = lib.types.int; };
        };
      };

      config.var = {
        opacity = 0.98;
        borderSize = 3;
        rounding = 10;
        fonts = {
          sans = "Rubik";
          mono = "JetBrainsMono Nerd Font";
          serif = "Rubik";
          interfaceSize = 15;
          codeSize = 12;
        };
      };
    };
}
