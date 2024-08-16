{
  pkgs,
  config,
  outputs,
  inputs,
  lib,
  ...
}:
let
  ifTheyExist = groups: builtins.filter (group: builtins.hasAttr group config.users.groups) groups;
  hostName = config.networking.hostName;
in
{
  users = {
    mutableUsers = false;
    users = {
      krezh = {
        hashedPasswordFile = config.sops.secrets.krezh-password.path;
        isNormalUser = true;
        shell = pkgs.fish;
        extraGroups =
          [
            "wheel"
            "video"
            "audio"
          ]
          ++ ifTheyExist [
            "network"
            "networkmanager"
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

  services.tailscale.enable = true;

  home-manager = {
    backupFileExtension = "bk";
    extraSpecialArgs = {
      inherit inputs outputs hostName;
    };
    useGlobalPkgs = true;
    useUserPackages = true;
    users = {
      # Import your home-manager configuration
      krezh = import ../../../../home/krezh;
    };
  };

  environment = {
    noXlibs = lib.mkForce true;
  };
}
