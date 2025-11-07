{ inputs, ... }:
{
  imports = [
    inputs.catppuccin.homeModules.catppuccin
  ];

  catppuccin = {
    enable = true;
    flavor = "mocha";
    accent = "blue";
  };

  catppuccin.cursors.enable = true;
  catppuccin.cursors.flavor = "mocha";
  catppuccin.cursors.accent = "light";
}
