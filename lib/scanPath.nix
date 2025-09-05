{ inputs, ... }:
let
  lib = inputs.nixpkgs.lib;

  # Common filtering logic for both functions
  # Filters out default.nix, excluded files, and keeps only directories and .nix files
  filterValidEntries =
    excludeFiles: entries:
    lib.attrsets.filterAttrs (
      name: type:
      !(builtins.elem name ([ "default.nix" ] ++ excludeFiles))
      && (type == "directory" || lib.strings.hasSuffix ".nix" name)
    ) entries;

  # Get filtered file/directory names from a path
  # Throws error if path doesn't exist or isn't readable
  getValidNames =
    path: excludeFiles:
    let
      pathStr = toString path;
    in
    if !builtins.pathExists path then
      throw "scanPath: Path does not exist: ${pathStr}"
    else
      builtins.attrNames (filterValidEntries excludeFiles (builtins.readDir path));

  # Convert filename to attribute name (remove .nix suffix)
  fileToAttrName =
    filename:
    if lib.strings.hasSuffix ".nix" filename then
      lib.strings.removeSuffix ".nix" filename
    else
      filename;
in
{
  # Scans a directory and returns a list of paths to .nix files and subdirectories
  #
  # Arguments:
  #   path: Path to scan (can be string or path type)
  #   excludeFiles: List of filenames to exclude (default: [])
  #
  # Returns: List of paths as strings
  #
  # Example: toList { path = ./modules; excludeFiles = ["broken.nix"]; }
  toList =
    {
      path,
      excludeFiles ? [ ],
    }:
    builtins.map (name: path + "/${name}") (getValidNames path excludeFiles);

  # Scans directories and returns an attribute set by calling a function on each path
  #
  # Arguments:
  #   paths: Single path spec or list of path specs
  #          Each path spec should have: { path = ./some/path; func = import; }
  #   excludeFiles: List of filenames to exclude (default: [])
  #   args: Arguments to pass to the function (default: {})
  #
  # Returns: Attribute set where keys are filenames (without .nix) and values are function results
  #
  # Example: toAttrs {
  #   paths = { path = ./modules; func = import; };
  #   args = { inherit pkgs; };
  # }
  toAttrs =
    {
      paths,
      excludeFiles ? [ ],
      args ? { },
    }:
    let
      pathsList = if builtins.isList paths then paths else [ paths ];
      processPath =
        pathSpec:
        if !(pathSpec ? path && pathSpec ? func) then
          throw "scanPath.toAttrs: Each path spec must have 'path' and 'func' attributes"
        else
          builtins.map (filename: {
            name = fileToAttrName filename;
            value = pathSpec.func (pathSpec.path + "/${filename}") args;
          }) (getValidNames pathSpec.path excludeFiles);
    in
    builtins.listToAttrs (builtins.concatMap processPath pathsList);
}
