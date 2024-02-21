# This is your system's configuration file.
{ inputs, outputs, modulesPath, lib, config, pkgs, ... }:
{
  imports = [
    (modulesPath + "/profiles/minimal.nix")
    inputs.disko.nixosModules.disko

    ../common/global
    ../common/users/krezh
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

  boot.loader.grub.devices = [ "/dev/nvme0n1" ];
  boot.loader.grub.enable = true;
  boot.loader.grub.efiSupport = true;
  boot.kernelPackages = pkgs.linuxPackages_latest;
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

  security = {
    sudo.wheelNeedsPassword = true;
  };

  networking.hostName = "odin";

  # https://nixos.wiki/wiki/FAQ/When_do_I_update_stateVersion
  system.stateVersion = "23.11";
}
