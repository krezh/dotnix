{
  inputs,
  ...
}:
{
  imports = [
    inputs.nix-index.homeModules.nix-index
  ];

  programs.nix-index.enable = true;
}
