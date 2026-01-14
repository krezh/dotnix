{
  flake.modules.homeManager.aria2.programs.aria2 = {
    enable = true;
    settings = {
      file-allocation = "none";
      log-level = "warn";
      max-connection-per-server = 16;
      min-split-size = "1M";
      human-readable = true;
      continue = true;
      split = 16;
      disk-cache = "32M";
    };
  };
}
