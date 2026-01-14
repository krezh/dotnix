{
  inputs,
  ...
}:
let
  username = "krezh";
in
{
  flake.modules.nixos.${username} =
    {
      pkgs,
      config,
      ...
    }:
    let
      ifTheyExist = groups: builtins.filter (group: builtins.hasAttr group config.users.groups) groups;
    in
    {
      # Home-manager integration
      home-manager = {
        users.${username} = {
          imports = [ inputs.self.modules.homeManager.${username} ];
        };
      };

      # User account
      users = {
        mutableUsers = true;
        users.${username} = {
          initialPassword = username;
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
}
