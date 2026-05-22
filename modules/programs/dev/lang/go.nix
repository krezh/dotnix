{
  flake.modules.homeManager.go =
    { pkgs, config, ... }:
    {
      home.packages = with pkgs; [
        gopls
      ];
      programs.go = {
        enable = true;
        env.GOPATH = "${config.xdg.dataHome}/go";
        packages = { };
      };
    };
}
