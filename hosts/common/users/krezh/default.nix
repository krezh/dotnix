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
    mutableUsers = true;
    users = {
      krezh = {
        initialPassword = "krezh";
        isNormalUser = true;
        shell = pkgs.unstable.fish;
        openssh.authorizedKeys.keys = [
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIANNodE0rg2XalK+tfsqfPwLdBRJIx15IjGwkr5Bud+W"
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEMe4X4oNA8PRUHrOk5RIrpxpzzcBvJyQa8PyaQj3BPp"
        ];
        extraGroups = [
          "wheel"
          "video"
          "audio"
          "sshusers"
          "input"
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
          "gamemode"
        ];
      };
    };
  };
  nixosModules.desktop.smb-mount = {
    enable = true;
    server = "jotunheim.srv.plexuz.xyz";
    share = "homes"; # The SMB share name
    mountPoint = "/mnt/home";
    credentialsFile = config.sops.templates."jotunheim_homes_creds".path;
  };
}
