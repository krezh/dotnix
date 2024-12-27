{ pkgs, inputs, ... }:
{
  imports = [ ];
  environment.systemPackages = with pkgs; [
    wget
    git
    deadnix
    usbutils
    nix-init
    nix-update
    inputs.nixd.packages.${pkgs.system}.nixd
    nix-inspect
  ];

  programs.nh = {
    enable = true;
  };
}
