# This file (and the global directory) holds config that i use on all hosts
{
  inputs,
  pkgs,
  lib,
  outputs,
  ...
}:
{
  imports = [
    inputs.home-manager.nixosModules.home-manager
  ] ++ (builtins.attrValues outputs.nixosModules);

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
  xdg.terminal-exec.enable = true;
  xdg.terminal-exec.settings = {
    default = [
      "kitty.desktop"
    ];
  };

  zramSwap.enable = true;

  services.pcscd.enable = true;

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

  system.stateVersion = lib.mkDefault "24.05";
}
