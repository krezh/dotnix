{
  flake.modules.nixos.thor =
    { config, ... }:
    {
      home-manager.users.krezh = {
        homeModules.snappy-switcher = {
          enable = false;
          settings.font.family = config.var.fonts.sans;
        };
      };
    };
}
