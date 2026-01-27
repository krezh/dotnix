{ inputs, ... }:
{
  flake.modules.homeManager.catppuccin = {
    imports = [
      inputs.catppuccin.homeModules.catppuccin
    ];

    catppuccin = {
      enable = true;
      flavor = "mocha";
      accent = "blue";
      cursors = {
        enable = true;
        flavor = "mocha";
        accent = "light";
      };
    };
  };
}
