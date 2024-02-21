{ pkgs, config, outputs, inputs, lib, ... }:
let
  ifTheyExist = groups:
    builtins.filter (group: builtins.hasAttr group config.users.groups) groups;
in
{
  users = {
    mutableUsers = false;
    users = {
      krezh = {
        initialPassword = "password";
        isNormalUser = true;
        shell = pkgs.fish;
        extraGroups = [ "wheel" "video" "audio" ] ++ ifTheyExist [
          "minecraft"
          "network"
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

  home-manager = {
    extraSpecialArgs = { inherit inputs outputs; };
    useGlobalPkgs = true;
    useUserPackages = true;
    users = {
      # Import your home-manager configuration
      krezh = import ../../../../home/krezh;
    };
  };

  environment = {
    noXlibs = lib.mkForce false;
    etc = lib.mapAttrs'
      (name: value: {
        name = "nix/path/${name}";
        value.source = value.flake;
      })
      config.nix.registry;
    systemPackages = with pkgs; [
      wget
      git
      talosctl
    ];
  };
}
