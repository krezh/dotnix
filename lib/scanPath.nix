{ lib, ... }:
let
  # Pattern matching - regex patterns
  matchesRegex = pattern: str: (builtins.match pattern str) != null;

  matchesAny = patterns: name: builtins.any (p: matchesRegex p name) patterns;

  # Check if name starts with underscore
  startsWithUnderscore = name: builtins.substring 0 1 name == "_";

  # Multi-stage filtering: cheap checks first, expensive checks last
  shouldExclude =
    cfg: name:
    # Stage 0: Underscore prefix check
    startsWithUnderscore name
    # Stage 1: Direct name match
    || builtins.elem name cfg.excludeFiles
    # Stage 2: Pattern matching
    || matchesAny cfg.excludePatterns name;

  shouldInclude = cfg: name: cfg.includePatterns == [ ] || matchesAny cfg.includePatterns name;

  isValidType =
    cfg: name: type:
    type == "directory" || builtins.any (ext: lib.strings.hasSuffix ext name) cfg.extensions;

  # Memoized readDir - caches directory reads
  cachedReadDir =
    cache: basePath:
    let
      key = toString basePath;
    in
    if cache ? ${key} then
      {
        inherit cache;
        entries = cache.${key};
      }
    else
      let
        entries = builtins.readDir basePath;
      in
      {
        cache = cache // {
          ${key} = entries;
        };
        inherit entries;
      };

  # Core recursive scanner with all optimizations
  scan =
    {
      basePath,
      config,
      collected ? [ ],
      processed ? { },
      cache ? { },
      rootPath ? basePath, # Track the original root path
    }:
    let
      key = toString basePath;
    in
    # Early termination: already processed
    if processed ? ${key} then
      {
        paths = collected;
        inherit cache;
      }
    # Validation: basePath must exist
    else if !(builtins.pathExists basePath) then
      throw ''
        scanPath: Path not found
          Path: ${key}

          The referenced path does not exist on the filesystem.
          Check that the path is correct and accessible.
      ''
    else
      let
        # Memoized directory read
        readResult = cachedReadDir cache basePath;
        inherit (readResult) entries;
        newCache = readResult.cache;

        # Filter entries
        validEntries = lib.filterAttrs (
          name: type: !shouldExclude config name && isValidType config name type && shouldInclude config name
        ) entries;

        # Mark as processed
        newProcessed = processed // {
          ${key} = true;
        };

        # Process each entry
        processEntry =
          acc: name:
          let
            entryPath = basePath + "/${name}";
            entryType = validEntries.${name};
          in
          if entryType == "directory" then
            let
              # Check for default.nix (memoized)
              dirRead = cachedReadDir acc.cache entryPath;
              hasDefault = dirRead.entries ? "default.nix";
            in
            if hasDefault then
              # Module directory - don't recurse
              {
                paths = acc.paths ++ [ entryPath ];
                inherit (dirRead) cache;
              }
            else
              # Regular directory - recurse
              scan {
                basePath = entryPath;
                inherit config rootPath;
                collected = acc.paths;
                processed = newProcessed;
                inherit (dirRead) cache;
              }
          else
            # Nix file
            {
              paths = acc.paths ++ [ entryPath ];
              inherit (acc) cache;
            };

        result = builtins.foldl' processEntry {
          paths = collected;
          cache = newCache;
        } (builtins.attrNames validEntries);
      in
      result;
in
{
  # Simple form: just pass a basePath directly
  # Uses lazy evaluation to avoid self-import issues
  toImports =
    basePath:
    map (basePath: { imports = [ basePath ]; })
      (scan {
        inherit basePath;
        config = {
          extensions = [ ".nix" ];
          excludeFiles = [ ];
          excludePatterns = [ ];
          includePatterns = [ ];
        };
      }).paths;

  # Return list of discovered paths
  toList =
    {
      basePath,
      extensions ? [ ".nix" ],
      excludeFiles ? [ ],
      excludePatterns ? [ ],
      includePatterns ? [ ],
    }:
    (scan {
      inherit basePath;
      config = {
        inherit
          extensions
          excludeFiles
          excludePatterns
          includePatterns
          ;
      };
    }).paths;

  # Return attribute set by calling func on each path
  toAttrs =
    {
      basePath,
      func,
      args ? { },
      useBaseName ? false,
      extensions ? [ ".nix" ],
      excludeFiles ? [ ],
      excludePatterns ? [ ],
      includePatterns ? [ ],
    }:
    let
      inherit
        (
          (scan {
            inherit basePath;
            config = {
              inherit
                extensions
                excludeFiles
                excludePatterns
                includePatterns
                ;
            };
          })
        )
        paths
        ;

      rootStr = toString basePath;

      # Remove any matching extension from a filename
      removeExtension =
        filename:
        let
          matchingExt = lib.findFirst (ext: lib.strings.hasSuffix ext filename) null extensions;
        in
        if matchingExt != null then lib.removeSuffix matchingExt filename else filename;

      toAttr =
        nixPath:
        let
          pathStr = toString nixPath;
          relPath = lib.removePrefix (rootStr + "/") pathStr;

          name =
            if useBaseName then
              removeExtension (baseNameOf nixPath)
            else
              removeExtension (builtins.replaceStrings [ "/" ] [ "_" ] relPath);
        in
        {
          inherit name;
          value = func nixPath args;
        };
    in
    builtins.listToAttrs (map toAttr paths);
}
