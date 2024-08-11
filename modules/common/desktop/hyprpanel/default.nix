{
  config,
  pkgs,
  lib,
  inputs,
  ...
}:
let
  cfg = config.modules.desktop.hyprpanel;
  hyprpanelFlake = inputs.hyprpanel.packages.${pkgs.system}.default;
in
{
  options.modules.desktop.hyprpanel = {
    enable = lib.mkEnableOption "hyprpanel";

    package = lib.mkPackageOption pkgs hyprpanelFlake { };

    settings = lib.mkOption {
      type = lib.types.attrs;
      default = { };
    };
  };

  config = lib.mkIf cfg.enable {
    home = {
      packages = [ hyprpanelFlake ];
      sessionVariables = { };
    };

    services.dunst.enable = lib.mkForce false; # Disable dunst if hyprpanel is enabled

    systemd.user.services.hyprpanel = {
      Unit = {
        Description = "A Solution to your Wayland Wallpaper Woes";
        Documentation = "https://github.com/Horus645/swww";
      };
      Service = {
        PassEnvironment = [
          "PATH"
          "XDG_RUNTIME_DIR"
        ];
        ExecStart = "${hyprpanelFlake}/bin/hyprpanel";
        Restart = "on-failure";
      };
      Install.WantedBy = [
        (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
      ];
    };
  };
}
