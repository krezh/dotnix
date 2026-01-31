{ inputs, ... }:
{
  flake.modules.nixos.catppuccin = {
    imports = [
      inputs.catppuccin.nixosModules.catppuccin
    ];

    catppuccin = {
      enable = true;
      flavor = "mocha";
      accent = "blue";
      cache.enable = true;
    };
  };
}
