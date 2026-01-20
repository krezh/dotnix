{inputs, ...}: {
  flake.modules.nixos.thor = {
    home-manager.users.krezh = {
      imports = [inputs.self.modules.homeManager.swww];
      services.swww-random = {
        enable = true;
        settings.interval = 60 * 10; # 10 minutes
      };
    };
  };
}
