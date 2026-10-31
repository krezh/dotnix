{
  flake.modules.nixos.thor =
    { config, ... }:
    {
      home-manager.users.krezh = {
        homeModules.snappy-switcher = {
          enable = true;
          settings.font.family = config.var.fonts.sans;
        };
      };
    };
}
