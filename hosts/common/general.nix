# This file (and the global directory) holds config that i use on all hosts
{
  inputs,
  pkgs,
  outputs,
  ...
}:
{
  imports = [
    inputs.sops-nix.nixosModules.sops
    inputs.home-manager.nixosModules.home-manager
    inputs.catppuccin.nixosModules.catppuccin
  ]
  ++ (builtins.attrValues outputs.nixosModules);

  system.stateVersion = "24.05";

  catppuccin = {
    enable = true;
    flavor = "mocha";
    accent = "blue";
    cache.enable = true;
  };

  environment.variables = {
    QT_QPA_PLATFORM = "wayland";
  };

  hardware.enableRedistributableFirmware = true;
  xdg.terminal-exec.enable = true;
  xdg.terminal-exec.settings.default = [ "kitty" ];

  services.pcscd.enable = true;

  security = {
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
        {
          commands = [
            {
              command = "/run/current-system/sw/bin/true";
              options = [ "NOPASSWD" ];
            }
          ];
          users = [ "root" ];
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
}
