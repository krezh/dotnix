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
  ];

  nix.settings = {
    trusted-users = [ "root" ];
    max-jobs = "auto";
    builders-use-substitutes = true;
    experimental-features = [
      "nix-command"
      "flakes"
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
