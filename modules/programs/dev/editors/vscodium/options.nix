{
  flake.modules.homeManager.editors =
    { lib, ... }:
    {
      options.vscodium.extensionIds = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
      };
    };
}
