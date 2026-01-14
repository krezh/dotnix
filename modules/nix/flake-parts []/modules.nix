{
  inputs,
  ...
}:
{
  imports = [
    inputs.flake-parts.flakeModules.modules
    inputs.devshell.flakeModule
    inputs.treefmt-nix.flakeModule
  ];
}
