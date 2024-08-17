{ inputs, ... }:
{
  imports = [ ./mapToGha.nix ];
  scanPath = import ./scanPath.nix { inherit inputs; };

  relativeToRoot = inputs.nixpkgs.lib.path.append ../.;

  mapToGha =
    system:
    if system == "x86_64-linux" then
      "ubuntu-latest"
    else if system == "x86_64-darwin" then
      "ubuntu-latest"
    else if system == "aarch64-darwin" then
      "macos-14.0"
    else
      system;
}
