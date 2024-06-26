{
  pkgs,
  config,
  ...
}:
{

  home = {
    packages = with pkgs; [ swww ];
  };

  # systemd.user.services.swww = {
  #   Unit = {
  #     Description = "swww";
  #     PartOf = [
  #       "tray.target"
  #       "graphical-session.target"
  #     ];
  #   };
  #   Service = {
  #     Environment = "PATH=/run/wrappers/bin:${lib.makeBinPath dependencies}";
  #     ExecStart = "${pkgs.swww}/bin/swww-daemon";
  #     Restart = "on-failure";
  #   };
  #   Install.WantedBy = [ "graphical-session.target" ];
  # };
}
