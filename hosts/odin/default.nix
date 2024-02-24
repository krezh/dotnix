# This is your system's configuration file.
{ inputs, outputs, modulesPath, lib, config, pkgs, ... }:
let
  hyprRun = pkgs.writeShellScript "hypr-run" ''
    export XDG_SESSION_TYPE=wayland
    export XDG_SESSION_DESKTOP=hyprland
    export XDG_CURRENT_DESKTOP=hyprland

    ${pkgs.hyprland}/bin/Hyprland
  '';
in
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


  services.greetd = {
    enable = true;
    settings = {
      default_session = {
        command = "${lib.makeBinPath [pkgs.greetd.tuigreet] }/tuigreet --time --cmd ${hyprRun}";
      };
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
