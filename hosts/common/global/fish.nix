{
  lib, ...
}:
{
  programs.fish = {
    enable = true;
    vendor = {
      completions.enable = true;
      config.enable = true;
      functions.enable = true;
    };
  };
  documentation.man.generateCaches = lib.mkForce false;
}
