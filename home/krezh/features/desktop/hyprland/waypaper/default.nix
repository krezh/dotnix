{ pkgs, ... }:
{
  home = {
    packages = with pkgs; [ waypaper ];
  };

  # systemd.user.services.waypaper = {
  #   Unit = {
  #     Description = "waypaper";
  #     PartOf = [
  #       "tray.target"
  #       "graphical-session.target"
  #     ];
  #   };
  #   Service = {
  #     Environment = "PATH=/run/wrappers/bin:${lib.makeBinPath dependencies}";
  #     ExecStart = "${pkgs.waypaper}/bin/waypaper --restore";
  #     Type = "oneshot";
  #   };
  #   Install.WantedBy = [ "graphical-session.target" ];
  # };
}
