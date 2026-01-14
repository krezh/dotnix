{ lib }:
final: prev:
let
  extensionsPath = ../pkgs/vscode-extensions;
  publishers = builtins.readDir extensionsPath;
  mkPublisher =
    name: _:
    lib.scanPath.toAttrs {
      basePath = extensionsPath + "/${name}";
      func = final.callPackage;
    };
in
{
  vscode-extensions = prev.vscode-extensions // lib.mapAttrs mkPublisher publishers;
}
