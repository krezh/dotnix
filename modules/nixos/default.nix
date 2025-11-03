{ lib, ... }:
{
  imports = [ (lib.importTree ./desktop) ];
}
