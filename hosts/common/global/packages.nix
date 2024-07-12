{ pkgs, ... }:
{
  environment = {
    systemPackages = with pkgs; [
      wget
      git
      deadnix
      nix-init
    ];
  };
}
