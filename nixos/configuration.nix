# This is your system's configuration file.
# Use this to configure your system environment (it replaces /etc/nixos/configuration.nix)
{
  inputs,
  outputs,
  modulesPath,
  lib,
  config,
  pkgs,
  ...
}: {
  # You can import other NixOS modules here
  imports = [
    # include NixOS-WSL modules
    inputs.nixos-wsl.nixosModules.wsl
    (modulesPath + "/profiles/minimal.nix")
    inputs.home-manager.nixosModules.home-manager
    # If you want to use modules from other flakes (such as nixos-hardware):
    # inputs.hardware.nixosModules.common-cpu-amd
    # inputs.hardware.nixosModules.common-ssd

    # You can also split up your configuration and import pieces of it here:
    # ./users.nix

    # Import your generated (nixos-generate-config) hardware configuration
    ./hardware-configuration.nix
  ];

  wsl = {
    enable = true;
    defaultUser = "krezh";
    nativeSystemd = true;
    wslConf.network = {
      hostname = "nixos";
      generateResolvConf = true;
    };
    startMenuLaunchers = false;
    interop.includePath = false;
  };

  home-manager = {
    extraSpecialArgs = { inherit inputs outputs; };
    users = {
      # Import your home-manager configuration
      krezh = import ../home-manager/home.nix;
    };
  };

  security.sudo.wheelNeedsPassword = true;
  nix.gc = {
    automatic = true;
    dates = "weekly";
    options = "--delete-older-than 7d";
  };

  environment = {
    noXlibs = lib.mkForce false;
    systemPackages = with pkgs; [
      wget
      wslu
      git
      neovim
    ];
  };

  # doesn't work on wsl
  services.dbus.apparmor = "disabled";
  # ditto
  networking.networkmanager.enable = false;

  boot.isContainer = true;

  ## Disable systemd units that don't make sense on WSL
  #systemd.services."serial-getty@ttyS0".enable = false;
  #systemd.services."serial-getty@hvc0".enable = false;
  #systemd.services."getty@tty1".enable = false;
  #systemd.services."autovt@".enable = false;

  #systemd.services.firewall.enable = false;
  #systemd.services.systemd-resolved.enable = false;
  #systemd.services.systemd-udevd.enable = false;

  ## Don't allow emergency mode, because we don't have a console.
  #systemd.enableEmergencyMode = false;

  # ditto
  security = {
    apparmor.enable = false;
    audit.enable = false;
    auditd.enable = false;
  };

  # ditto
  services.resolved.enable = false;

  # This will add each flake input as a registry
  # To make nix3 commands consistent with your flake
  nix.registry = (lib.mapAttrs (_: flake: {inherit flake;})) ((lib.filterAttrs (_: lib.isType "flake")) inputs);

  # This will additionally add your inputs to the system's legacy channels
  # Making legacy nix commands consistent as well, awesome!
  nix.nixPath = ["/etc/nix/path"];
  environment.etc =
    lib.mapAttrs'
    (name: value: {
      name = "nix/path/${name}";
      value.source = value.flake;
    })
    config.nix.registry;

  nix.settings = {
    # Enable flakes and new 'nix' command
    experimental-features = "nix-command flakes";
    # Deduplicate and optimize nix store
    auto-optimise-store = true;
  };

  # FIXME: Add the rest of your current configuration

  # TODO: Set your hostname
  networking.hostName = "nixos";

  # TODO: Configure your system-wide user settings (groups, etc), add more users as needed.
  users.users = {
    # FIXME: Replace with your username
    krezh = {
      # TODO: You can set an initial password for your user.
      # If you do, you can skip setting a root password by passing '--no-root-passwd' to nixos-install.
      # Be sure to change it (using passwd) after rebooting!
      initialPassword = "krezh";
      isNormalUser = true;
      # TODO: Be sure to add any other groups you need (such as networkmanager, audio, docker, etc)
      extraGroups = ["wheel"];
    };
  };

  # https://nixos.wiki/wiki/FAQ/When_do_I_update_stateVersion
  system.stateVersion = "23.11";
}
