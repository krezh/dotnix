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

  boot.kernelPackages = pkgs.linuxPackages_latest;
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;
  boot.loader.grub.device = "nodev";
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
