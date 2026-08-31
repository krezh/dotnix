{
  flake.modules.nixos.thor = {
    home-manager.users.krezh = {
      homeModules.snappy-switcher.enable = true;
    };
  };
}
