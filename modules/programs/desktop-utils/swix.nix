_: {
  flake.modules.nixos.desktop-utils =
    { config, ... }:
    {
      nixosModules.swix = {
        enable = true;
        user = config.var.username;
        settings = {
          appearance = {
            sansFont = config.var.fonts.sans;
            monoFont = config.var.fonts.mono;
            rounding = config.var.rounding;
          };
          flakeDir = "/home/${config.var.username}/dotnix";
          nixosFlake = config.networking.hostName;
        };
      };
    };
}
