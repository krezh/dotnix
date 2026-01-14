{
  flake.modules.homeManager.mail =
    { pkgs, ... }:
    {
      home.packages = [ pkgs.mailspring ];
    };
}
