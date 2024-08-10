{ pkgs, inputs, ... }:
{
  imports = [ ];
  environment.systemPackages = with pkgs; [
    wget
    git
    deadnix
    nix-init
    inputs.nixd.packages.${pkgs.system}.nixd
    inputs.nix-update.packages.${pkgs.system}.nix-update
    inputs.nix-inspect.packages.${pkgs.system}.default
  ];

  # programs.winbox = {
  #   enable = true;
  #   openFirewall = true;
  # };

  programs.nh = {
    enable = true;
    package = inputs.nh.packages.${pkgs.system}.default;
  };
}
