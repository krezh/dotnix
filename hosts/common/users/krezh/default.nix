{
  pkgs,
  config,
  lib,
  ...
}:
let
  ifTheyExist = groups: builtins.filter (group: builtins.hasAttr group config.users.groups) groups;
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
            "sshusers"
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

  environment = {
    noXlibs = lib.mkForce false;
  };
}
