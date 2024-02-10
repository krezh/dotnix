# This is your system's configuration file.
{ inputs, outputs, modulesPath, lib, config, pkgs, ... }: {
  # You can import other NixOS modules here
  imports = [
    (modulesPath + "/profiles/minimal.nix")
    inputs.home-manager.nixosModules.home-manager

    ../common/global
    ./hardware-configuration.nix
  ];

  nixpkgs = {
    # You can add overlays here
    overlays = [
      # Add overlays your own flake exports (from overlays and pkgs dir):
      outputs.overlays.additions
      outputs.overlays.modifications
      outputs.overlays.unstable-packages
    ];
  };

  home-manager = {
    extraSpecialArgs = { inherit inputs outputs; };
    # useGlobalPkgs = true;
    # useUserPackages = true;
    users = {
      # Import your home-manager configuration
      krezh = import ../../home/krezh;
    };
  };

  environment = {
    noXlibs = lib.mkForce false;
    systemPackages = with pkgs; [ wget wslu git neovim ];
  };

  boot.isContainer = true;
  security.sudo.wheelNeedsPassword = true;

  # doesn't work on wsl
  services.dbus.apparmor = "disabled";
  services.resolved.enable = false;
  networking.networkmanager.enable = false;
  security = {
    apparmor.enable = false;
    audit.enable = false;
    auditd.enable = false;
  };

  environment.etc = lib.mapAttrs'
    (name: value: {
      name = "nix/path/${name}";
      value.source = value.flake;
    })
    config.nix.registry;

  networking.hostName = "thor-wsl";

  programs.fish.enable = true;
  users.users = {
    krezh = {
      initialPassword = "krezh";
      isNormalUser = true;
      extraGroups = [ "wheel" ];
      shell = pkgs.fish;
    };
  };

  # https://nixos.wiki/wiki/FAQ/When_do_I_update_stateVersion
  system.stateVersion = "23.11";
}
