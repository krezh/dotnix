{
  inputs,
  self,
  lib,
  ...
}:
let
  inherit (self) outputs;

  flakeLib = import ../lib/flakeLib.nix {
    inherit
      inputs
      outputs
      lib
      self
      ;
  };
in
{
  flake = {
    nixosConfigurations = flakeLib.mkSystems {
      thor = {
        system = "x86_64-linux";
        homeUsers = [ "krezh" ];
      };
      odin = {
        system = "x86_64-linux";
        homeUsers = [ "krezh" ];
      };
      nixos-livecd = {
        system = "x86_64-linux";
        homeUsers = [ ];
        commonHost = false;
        desktop = false;
      };
    };

    ghMatrix = flakeLib.ghMatrix { exclude = [ "nixos-livecd" ]; };
    top = flakeLib.top;

    overlays = import ../overlays { inherit inputs lib; };
    homeManagerModules = lib.scanPath.toList { path = ../modules/homeManager; };
    nixosModules.default = lib.scanPath.toImports ../modules/nixos;
  };
}
