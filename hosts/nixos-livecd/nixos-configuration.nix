{
  modulesPath,
  pkgs,
  hostname,
  inputs,
  ...
}:
{
  imports = [ (modulesPath + "/installer/cd-dvd/installation-cd-minimal.nix") ];

  config = {
    # faster build time
    isoImage.squashfsCompression = "gzip -Xcompression-level 1";

    # enable SSH in the boot process
    systemd.services.sshd.wantedBy = pkgs.lib.mkForce [ "multi-user.target" ];
    users.users.nixos.openssh.authorizedKeys.keys = [
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIANNodE0rg2XalK+tfsqfPwLdBRJIx15IjGwkr5Bud+W"
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEMe4X4oNA8PRUHrOk5RIrpxpzzcBvJyQa8PyaQj3BPp"
    ];

    environment.systemPackages = [
      pkgs.neovim
      pkgs.gitMinimal
      pkgs.sops
      pkgs.age-plugin-yubikey
      inputs.disko.packages.${pkgs.stdenv.hostPlatform.system}.disko-install
      inputs.disko.packages.${pkgs.stdenv.hostPlatform.system}.disko
    ];

    networking.hostName = hostname;

    time.timeZone = "Europe/Stockholm";
    console.keyMap = "sv-latin1";

    security = {
      sudo.wheelNeedsPassword = false;
    };
  };
}
