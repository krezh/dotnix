{ inputs, lib, pkgs, config, ... }:
let
  requiredDeps = with pkgs; [ config.wayland.windowManager.hyprland.package ];
  guiDeps = with pkgs; [ inputs.swww.packages.${pkgs.system}.swww ];
  dependencies = requiredDeps ++ guiDeps;
in {
  systemd.user.services.swww = {
    Unit = {
      Description = "swww";
      PartOf = [ "tray.target" "graphical-session.target" ];
    };
    Service = {
      Environment = "PATH=/run/wrappers/bin:${lib.makeBinPath dependencies}";
      ExecStart = "${pkgs.swww}/bin/swww-daemon";
      Restart = "on-failure";
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };
}
