{
  flake.modules.homeManager.desktop-shell =
    { config, lib, ... }:
    let
      dusk = lib.getExe config.homeModules.dusk.package;
    in
    {
      homeModules.island = {
        enable = true;
        opacity = config.var.opacity;
        radius = config.var.rounding;
        session = {
          confirm = false;
          commands = {
            logout = [
              dusk
              "logout"
            ];
            reboot = [
              dusk
              "reboot"
            ];
            shutdown = [
              dusk
              "shutdown"
            ];
          };
        };
      };
    };
}
