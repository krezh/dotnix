{ pkgs, lib, ... }:

lib.scanPath.toAttrs {
  paths = [
    {
      path = ./bin;
      func = pkgs.callPackage;
      args = { };
    }
    {
      path = ./scripts;
      func = pkgs.callPackage;
      args = { };
    }
    # {
    #   path = ./fish-plugins;
    #   func = pkgs.fishPlugins.callPackage;
    #   args = { };
    # }
  ];
}
