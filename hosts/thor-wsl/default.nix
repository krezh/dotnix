# This is your system's configuration file.
{ inputs, outputs, modulesPath, lib, config, pkgs, ... }:
let
  ifTheyExist = groups:
    builtins.filter (group: builtins.hasAttr group config.users.groups) groups;
in
{
  # You can import other NixOS modules here
  imports = [
    # include NixOS-WSL modules
    inputs.nixos-wsl.nixosModules.wsl
    #inputs.sops-nix.nixosModules.sops
    (modulesPath + "/profiles/minimal.nix")
    inputs.home-manager.nixosModules.home-manager

    ../common/global
    ./hardware-configuration.nix
  ];

  wsl = {
    enable = true;
    defaultUser = "krezh";
    nativeSystemd = true;
    wslConf.network = {
      hostname = "thor-wsl";
      generateResolvConf = true;
    };
    startMenuLaunchers = false;
    interop.includePath = false;
  };

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
    useGlobalPkgs = true;
    useUserPackages = true;
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
  networking.hostName = "thor-wsl";
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

  programs.fish.enable = true;
  users = {
    mutableUsers = false;
    
    users = {
      krezh = {
        hashedPasswordFile = config.sops.secrets.krezh-password.path;
        isNormalUser = true;
        shell = pkgs.fish;
        extraGroups = [ "wheel" "video" "audio" ] ++ ifTheyExist [
          "minecraft"
          "network"
          "wireshark"
          "i2c"
          "mysql"
          "docker"
          "podman"
          "git"
          "libvirtd"
          "deluge"
        ];
      };
    };
  };

  # https://nixos.wiki/wiki/FAQ/When_do_I_update_stateVersion
  system.stateVersion = "23.11";
}
