{
  flake.modules.nixos.battery = {
    services = {
      upower.enable = true;
      power-profiles-daemon.enable = true;
    };
  };
}
