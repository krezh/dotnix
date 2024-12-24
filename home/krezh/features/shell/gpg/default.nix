{
  pkgs,
  config,
  lib,
  ...
}:
{
  services.gpg-agent = {
    enable = true;
    enableBashIntegration = true;
    enableFishIntegration = true;
    enableSshSupport = true;
    enableScDaemon = true;
    pinentryPackage = pkgs.pinentry-tty;
    enableExtraSocket = true;
  };

  programs = {
    gpg = {
      enable = true;
      scdaemonSettings = {
        disable-ccid = true;
        reader-port = "Yubico Yubi";
      };
    };
    home-manager.enable = true;
    neomutt.enable = true;
    yazi.enable = true;
  };
  home = {
    homeDirectory = lib.mkDefault "/home/${config.home.username}";
    stateVersion = lib.mkDefault "24.05";
    preferXdgDirectories = true;
    sessionPath = [
      "$HOME/.local/bin"
      "$GOPATH/bin"
      "$CARGO_HOME/bin"
    ];
    sessionVariables = {
      NH_FLAKE = "${config.home.homeDirectory}/nix-config";
      GOPATH = "${config.xdg.dataHome}/go";
      CARGO_HOME = "${config.xdg.dataHome}/cargo";
      SOPS_AGE_KEY_FILE = "${config.sops.age.keyFile}";
    };
    packages = with pkgs; [
      gpg-tui
    ];
  };
}
