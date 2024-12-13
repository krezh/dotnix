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
    users.users.nixos.openssh.authorizedKeys.keys = [ inputs.ssh-keys.outPath ];

    environment.systemPackages = [
      pkgs.neovim
      pkgs.gitMinimal
    ];

    nix = {
      settings = {
        experimental-features = [
          "nix-command"
          "flakes"
        ];
      };
    };

    networking.hostName = hostname;

    time.timeZone = "Europe/Stockholm";
    console.keyMap = lib.mkDefault "sv-latin1";

    security = {
      sudo.wheelNeedsPassword = false;
    };
  };
}
