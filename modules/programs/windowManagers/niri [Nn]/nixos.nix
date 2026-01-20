{inputs, ...}: {
  flake.modules.nixos.niri = {pkgs, ...}: {
    nixpkgs.overlays = [
      inputs.niri.overlays.niri
    ];
    programs.niri.enable = true;
    programs.niri.package = pkgs.niri-unstable;
  };
}
