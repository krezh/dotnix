_: {
  flake.modules.nixos.desktop-utils =
    { config, ... }:
    {
      nixosModules.molt = {
        enable = true;
        user = config.var.username;
        settings = {
          appearance = {
            sansFont = config.var.fonts.sans;
            monoFont = config.var.fonts.mono;
            rounding = config.var.rounding;
          };
          flakeDir = "/home/${config.var.username}/dotnix";
          disableKeybinds = true;
          nixosFlake = config.networking.hostName;
        };
      };
    };
}
