{
  inputs,
  ...
}:
{
  scanPath = import ./scanPath.nix { inherit inputs; };

  relativeToRoot = inputs.nixpkgs.lib.path.append ../.;

  mapToGha =
    system:
    if system == "x86_64-linux" then
      "ubuntu-latest"
    else if system == "x86_64-darwin" then
      "ubuntu-latest"
    else if system == "aarch64-linux" then
      "ubuntu-24.04-arm"
    else
      system;
}
