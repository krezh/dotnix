{
  inputs,
  ...
}:
let
  imageConfigs = {
    livecd = {
      system = "x86_64-linux";
      specialArgs = {
        hostname = "livecd";
        inherit inputs;
      };
      modules = [
        ../../../images/livecd.nix
      ];
    };
  };
in
{
  flake.images = builtins.mapAttrs (
    _name: config: (inputs.nixpkgs.lib.nixosSystem config).config.system.build.images.iso-installer
  ) imageConfigs;
}
