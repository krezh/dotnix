{
  inputs,
  self,
  ...
}:
let
  inherit (self) outputs;
  lib = inputs.nixpkgs.lib // import ../lib { inherit inputs; };

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
    homeManagerModules = [ (inputs.import-tree ../modules/homeManager) ];
    nixosModules.default.imports = [ (inputs.import-tree ../modules/nixos) ];
  };
}
