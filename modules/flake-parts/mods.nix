{ lib, config, ... }:
{
  options.flake.mods = lib.mkOption {
    type = lib.types.attrsOf lib.types.unspecified;
    default = { };
  };

  config.flake.mods = lib.mapAttrs (_: lib.processModules) config.flake.modules;
}
