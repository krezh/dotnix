{
  pkgs,
  lib,
  modulesPath,
  config,
  ...
}:
{
  nixpkgs.hostPlatform = "x86_64-linux";

  system.build.kubevirtImage = lib.mkForce (
    import "${toString modulesPath}/../lib/make-disk-image.nix" {
      inherit lib config pkgs;
      inherit (config.image) baseName;
      format = "qcow2-compressed";
    }
  );

  users.users.root.openssh.authorizedKeys.keys = [
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIANNodE0rg2XalK+tfsqfPwLdBRJIx15IjGwkr5Bud+W"
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEMe4X4oNA8PRUHrOk5RIrpxpzzcBvJyQa8PyaQj3BPp"
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIBGOPhmqeY3GmqXDK2XHXM6csikAIVzBL+zziXB6LR2F remote-build"
  ];

  nix.settings = {
    trusted-users = [ "root" ];
    max-jobs = "auto";
    builders-use-substitutes = true;
    experimental-features = [
      "nix-command"
      "flakes"
    ];
    substituters = [
      "https://cache.nixos.org"
      "https://nix-cache.plexuz.xyz/krezh"
      "https://nix-community.cachix.org"
      "https://catppuccin.cachix.org"
    ];
    trusted-public-keys = [
      "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
      "krezh:bCYQVVbREhrYgC42zUMf99dMtVXIATXMCcq+wRimqCc="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "catppuccin.cachix.org-1:noG/4HkbhJb+lUAdKrph6LaozJvAeEEZj4N732IysmU="
    ];
  };

  environment.systemPackages = [
    pkgs.neovim
    pkgs.gitMinimal
  ];

  time.timeZone = "Europe/Stockholm";
  console.keyMap = "sv-latin1";

  system.stateVersion = "24.05";

  security = {
    sudo.wheelNeedsPassword = false;
  };
}
