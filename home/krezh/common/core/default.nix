{
  inputs,
  lib,
  config,
  osConfig,
  ...
}:
{
  imports = [
    inputs.nix-index.homeModules.nix-index
  ];

  xdg.enable = true;

  home = {
    homeDirectory = lib.mkDefault "/home/${config.home.username}";
    stateVersion = osConfig.system.stateVersion;
    preferXdgDirectories = true;
    sessionPath = [
      "$HOME/.local/bin"
      "$GOPATH/bin"
      "$CARGO_HOME/bin"
    ];
    sessionVariables = {
      FLAKE = "${config.home.homeDirectory}/nix-config";
      NH_FLAKE = "${config.home.homeDirectory}/nix-config";
      GOPATH = "${config.xdg.dataHome}/go";
      CARGO_HOME = "${config.xdg.dataHome}/cargo";
    };
  };

  programs = {
    home-manager.enable = true;
    yazi.enable = true;
    nix-index.enable = true;
  };

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";
}
