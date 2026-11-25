_: {
  flake.modules.nixos.desktop-utils =
    { config, ... }:
    {
      nixosModules.molt = {
        enable = true;
        user = config.var.username;
        settings = {
          flakeDir = "/home/${config.var.username}/dotnix";
          nixosFlake = config.networking.hostName;
        };
      };
    };
}
