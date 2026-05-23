{
  flake.modules.generic.var =
    { lib, ... }:
    {
      options.var.username = lib.mkOption { type = lib.types.str; };
    };
}
