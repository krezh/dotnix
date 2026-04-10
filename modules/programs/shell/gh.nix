_: {
  flake.modules.homeManager.shell =
    { pkgs, ... }:
    {
      programs.gh = {
        enable = true;
        extensions = with pkgs; [
          gh-dash
          gh-markdown-preview
          gh-notify
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
