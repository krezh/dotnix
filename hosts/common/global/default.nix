# This file (and the global directory) holds config that i use on all hosts
{ inputs, outputs, lib, pkgs, ... }: {
  imports = [
    inputs.home-manager.nixosModules.home-manager
    ./locale.nix
    ./nix.nix
    ./podman.nix
    ./fish.nix
    ./sops.nix
  ] ++ (builtins.attrValues outputs.nixosModules);

  nixpkgs = {
    overlays = builtins.attrValues outputs.overlays;
    config = {
      allowUnfree = true;
      allowUnfreePredicate = true;
    };
  };

  # Fix for qt6 plugins
  environment.profileRelativeSessionVariables = {
    QT_PLUGIN_PATH = [ "/lib/qt-6/plugins" ];
  };

  hardware.enableRedistributableFirmware = true;

  security.sudo = {
    wheelNeedsPassword = true;
    enable = true;
    extraRules = [{
      commands = [{
        command = "${pkgs.systemd}/bin/reboot";
        options = [ "NOPASSWD" ];
      }];
      groups = [ "wheel" ];
    }];
    extraConfig = ''
      Defaults pwfeedback
    '';
  };

  # Increase open file limit for sudoers
  security.pam.loginLimits = [
    {
      domain = "@wheel";
      item = "nofile";
      type = "soft";
      value = "524288";
    }
    {
      domain = "@wheel";
      item = "nofile";
      type = "hard";
      value = "1048576";
    }
  ];

  system = {
    stateVersion = "23.11";
    # Enable printing changes on nix build etc with nvd
    activationScripts.report-changes = ''
      PATH=$PATH:${lib.makeBinPath [ pkgs.nvd pkgs.nix ]}
      nvd diff $(ls -dv /nix/var/nix/profiles/system-*-link | tail -2)
    '';
  };

}
