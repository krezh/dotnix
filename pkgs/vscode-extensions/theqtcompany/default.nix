{ callPackage }:
{
  qt-core = callPackage ./qt-core.nix { };
  qt-qml = callPackage ./qt-qml.nix { };
  qt-ui = callPackage ./qt-ui.nix { };
}
