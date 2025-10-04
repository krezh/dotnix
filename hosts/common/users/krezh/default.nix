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
        shell = pkgs.fish;
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
  nixosModules.desktop.mount = {
    enable = true;
    mounts = {
      "jotunheim-homes" = {
        enable = true;
        type = "smb";
        server = "jotunheim.srv.plexuz.xyz";
        share = "homes";
        mountPoint = "/mnt/home";
        credentialsFile = config.sops.templates."jotunheim_homes_creds".path;
      };
      "jotunheim-kopia" = {
        enable = true;
        type = "nfs";
        server = "jotunheim.srv.plexuz.xyz";
        share = "/mnt/tank/kopia";
        mountPoint = "/mnt/kopia";
        nfsVersion = "4.2";
        autoMount = true;
        uid = 1000;
        gid = 100;
        extraOptions = [
          "proto=tcp"
          "rsize=8192"
          "wsize=8192"
        ];
      };
    };
  };
}
