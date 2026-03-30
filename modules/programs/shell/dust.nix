{
  flake.modules.homeManager.shell =
    { pkgs, ... }:
    {
      home.packages = [ pkgs.dust ];
    };
}
