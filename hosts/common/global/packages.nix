{ pkgs, inputs, ... }:
{
  environment = {
    systemPackages = with pkgs; [
      wget
      git
      inputs.deadnix.packages.${pkgs.system}.deadnix
    ];
  };
}
