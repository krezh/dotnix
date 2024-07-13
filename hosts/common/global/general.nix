# This file (and the global directory) holds config that i use on all hosts
{
  inputs,
  outputs,
  pkgs,
  lib,
  mylib,
  ...
}:
{
  imports = [
    inputs.home-manager.nixosModules.home-manager
    inputs.catppuccin.nixosModules.catppuccin

  ] ++ (builtins.attrValues outputs.nixosModules);

  nixpkgs = {
    overlays = builtins.attrValues outputs.overlays;
    config = {
      allowUnfree = true;
      allowUnfreePredicate = true;
    };
  };

  catppuccin = {
    enable = true;
    flavor = "mocha";
    accent = "lavender";
  };

  # Fix for qt6 plugins
  environment.profileRelativeSessionVariables = {
    QT_PLUGIN_PATH = [ "/lib/qt-6/plugins" ];
  };

  hardware.enableRedistributableFirmware = true;

  security = {
    doas = {
      enable = false;
      wheelNeedsPassword = true;
      extraRules = [
        {
          cmd = "${pkgs.systemd}/bin/reboot";
          groups = [ "wheel" ];
          noPass = true;
        }
        {
          groups = [ "wheel" ];
          keepEnv = true;
          persist = true;
        }
      ];
    };
    sudo = {
      enable = true;
      wheelNeedsPassword = true;
      extraRules = [
        {
          commands = [
            {
              command = "${pkgs.systemd}/bin/reboot";
              options = [ "NOPASSWD" ];
            }
          ];
          groups = [ "wheel" ];
        }
      ];
      extraConfig = ''
        # Show feedback when typing password
        Defaults pwfeedback

        # Set sudo timeout to value in minutes
        Defaults timestamp_timeout=15 
      '';
    };

    # Increase open file limit for sudoers
    pam.loginLimits = [
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
  };

  system = {
    stateVersion = lib.mkDefault "24.05";
    # Enable printing changes on nix build etc with nvd
    # activationScripts.report-changes = ''
    #   PATH=$PATH:${lib.makeBinPath [ pkgs.nvd pkgs.nix ]}
    #   nvd diff $(ls -dv /nix/var/nix/profiles/system-*-link | tail -2)
    # '';
  };
}
