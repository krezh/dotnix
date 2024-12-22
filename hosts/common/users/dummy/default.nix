{
  pkgs,
  config,
  ...
}:
let
  ifTheyExist = groups: builtins.filter (group: builtins.hasAttr group config.users.groups) groups;
in
{
  users = {
    mutableUsers = false;
    users = {
      dummy = {
        hashedPasswordFile = config.sops.secrets."passwords/dummy".path;
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
}
