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
    nixosConfigurations = flakeLib.mkSystems [
      {
        hostname = "thor";
        system = "x86_64-linux";
        homeUsers = [ "krezh" ];
      }
      {
        hostname = "odin";
        system = "x86_64-linux";
        homeUsers = [ "krezh" ];
      }
      {
        hostname = "nixos-livecd";
        system = "x86_64-linux";
        homeUsers = [ ];
        importCommon = false;
      }
    ];

    ghMatrix = flakeLib.ghMatrix { exclude = [ "nixos-livecd" ]; };
    top = flakeLib.top;

    overlays = import ../overlays { inherit inputs lib; };
    homeManagerModules = [ ../modules/homeManager ];
    nixosModules.default.imports = [ ../modules/nixos ];
  };
}
