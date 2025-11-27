{
  lib,
  config,
  osConfig,
  ...
}:
{
  xdg.enable = true;

  home = {
    homeDirectory = lib.mkDefault "/home/${config.home.username}";
    stateVersion = osConfig.system.stateVersion;
    preferXdgDirectories = true;
    sessionVariables = {
      FLAKE = "${config.home.homeDirectory}/nix-config";
      NH_FLAKE = "${config.home.homeDirectory}/nix-config";
    };
  };

  programs = {
    home-manager.enable = true;
    yazi.enable = true;
  };

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";
}
