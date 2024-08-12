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
      packages = [ cfg.package ];
      sessionVariables = { };
    };

    services.dunst.enable = lib.mkForce false; # Disable dunst if hyprpanel is enabled

    systemd.user.services.hyprpanel = {
      Unit = {
        Description = "A Bar/Panel for Hyprland with extensive customizability.";
        Documentation = "https://github.com/Jas-SinghFSU/HyprPanel";
      };
      Service = {
        PassEnvironment = [
          "PATH"
          "XDG_RUNTIME_DIR"
        ];
        ExecStart = "${cfg.package}/bin/hyprpanel";
        Restart = "on-failure";
      };
      Install.WantedBy = [
        (lib.mkIf config.wayland.windowManager.hyprland.systemd.enable "hyprland-session.target")
      ];
    };
  };
}
