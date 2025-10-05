{
  inputs,
  ...
}:
{
  relativeToRoot = inputs.nixpkgs.lib.path.append ../.;
}
