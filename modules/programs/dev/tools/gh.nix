{
  flake.modules.homeManager.dev-tools =
    { pkgs, ... }:
    {
      programs.gh = {
        enable = true;
        extensions = with pkgs; [
          gh-dash
          gh-markdown-preview
          gh-poi
          gh-enhance
        ];
        settings = {
          git_protocol = "ssh";
          prompt = "enabled";
        };
      };
    };
}
