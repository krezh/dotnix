{inputs, ...}:
inputs.nixpkgs.lib.extend (
  _final: _prev: {
    relativeToRoot = _prev.path.append ../.;
    scanPath = import ./scanPath.nix {lib = _prev;};
  }
)
