{
  pkgs,
  hostname,
  ...
}:
{
  nixpkgs.hostPlatform = "x86_64-linux";

  users.users.root.openssh.authorizedKeys.keys = [
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIANNodE0rg2XalK+tfsqfPwLdBRJIx15IjGwkr5Bud+W"
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEMe4X4oNA8PRUHrOk5RIrpxpzzcBvJyQa8PyaQj3BPp"
  ];

  nix.settings = {
    trusted-users = [ "root" ];
    max-jobs = "auto";
    builders-use-substitutes = true;
  };

  environment.systemPackages = [
    pkgs.neovim
    pkgs.gitMinimal
  ];

  networking.hostName = hostname;
  time.timeZone = "Europe/Stockholm";
  console.keyMap = "sv-latin1";

  system.stateVersion = "24.05";

  security = {
    sudo.wheelNeedsPassword = false;
  };
}
