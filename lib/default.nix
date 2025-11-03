{
  inputs,
  ...
}:
let
  lib = inputs.nixpkgs.lib;
in
{
  relativeToRoot = lib.path.append ../.;

  # Import-tree helper that imports all .nix files from a directory
  # Excludes paths with underscores by default (import-tree convention)
  importTree = path: inputs.import-tree path;

  # Import-tree with custom filtering
  importTreeFiltered = path: filter: (inputs.import-tree path).filter filter;

  # Auto-discover packages from a list of directories
  # Each directory should contain subdirectories with package definitions
  # Returns an attribute set of packages suitable for overlays
  discoverPackages =
    callPackage: dirs:
    let
      # Process a single directory
      processDir =
        dir:
        let
          entries = builtins.readDir dir;
          # Filter for directories (packages are in subdirectories)
          packageDirs = lib.filterAttrs (_name: type: type == "directory") entries;
          # Convert each directory to a package attribute
          makePackage = name: _: callPackage (dir + "/${name}") { };
        in
        lib.mapAttrs makePackage packageDirs;

      # Process all directories and merge results
      packageSets = map processDir dirs;
    in
    lib.foldl' (acc: set: acc // set) { } packageSets;
}
