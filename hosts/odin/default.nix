# This is your system's configuration file.
{ inputs, outputs, modulesPath, lib, config, pkgs, ... }:
{
  imports = [
    inputs.disko.nixosModules.disko
    inputs.hyprland.nixosModules.default

    ../common/global
    ../common/users/krezh
    ./hardware-configuration.nix
  ];

  # Use latest kernel
  boot.kernelPackages = pkgs.linuxPackages_latest;

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;
  boot.loader.grub.device = "nodev";
  networking.networkmanager.enable = true;

  disko.devices = {
    disk = {
      vdb = {
        device = "/dev/nvme0n1";
        type = "disk";
        content = {
          type = "gpt";
          partitions = {
            ESP = {
              type = "EF00";
              size = "1G";
              content = {
                type = "filesystem";
                format = "vfat";
                mountpoint = "/boot";
              };
            };
            root = {
              size = "100%";
              content = {
                type = "filesystem";
                format = "ext4";
                mountpoint = "/";
              };
            };
          };
        };
      };
    };
  };

  # https://nixos.wiki/wiki/Greetd
  # tweaked for Hyprland
  # ...
  # launches swaylock with exec-once in home/hyprland/hyprland.conf
  # ...
  # single user and single window manager
  # my goal here is auto-login with authentication
  # so I can declare my user and environment (Hyprland) in this config
  # my goal is NOT to allow user selection or environment selection at the the login screen
  # (which a login manager provides beyond just the authentication check)
  # so I don't need a login manager
  # I just launch Hyprland as krezh automatically, which starts swaylock (to authenticate)
  # I thought I needed a greeter, but I really don't
  # ...
  services.greetd = {
    enable = true;
    settings = rec {
      initial_session = {
        command = "${pkgs.hyprland}/bin/Hyprland";
        user = "krezh";
      };
      default_session = initial_session;
    };
  };


  programs.hyprland = {
    enable = true;
    package = inputs.hyprland.packages.${pkgs.system}.hyprland;
  };

  environment = {
    systemPackages = with pkgs; [
      inputs.hyprlock.packages.${pkgs.system}.hyprlock
    ];
  };

  security = {
    sudo.wheelNeedsPassword = true;
  };

  networking.hostName = "odin";
}
