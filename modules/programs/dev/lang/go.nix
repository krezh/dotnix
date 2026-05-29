{
  flake.modules.homeManager.go =
    { pkgs, config, ... }:
    {
      home.packages = with pkgs; [
        gopls
        golangci-lint
        golangci-lint-langserver
      ];
      programs.go = {
        enable = true;
        env = {
          GOPATH = "${config.xdg.dataHome}/go";
          CGO_ENABLED = "0";
        };
        packages = { };
      };
    };
}
