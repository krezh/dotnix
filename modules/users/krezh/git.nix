{
  flake.modules.homeManager.krezh =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    {
      programs = {
        git = {
          enable = true;
          signing.format = "ssh";
          includes = [
            {
              condition = "hasconfig:remote.*.url:ssh://git@codeberg.org/**";
              contents.user.email = "krezh@noreply.codeberg.org";
            }
          ];
          settings = {
            user = {
              name = "Krezh";
              email = "krezh@users.noreply.github.com";
              signingkey = config.sops.secrets."ssh/privkey".path;
            };
            alias = {
              lol = "log --graph --decorate --pretty=oneline --abbrev-commit";
              lola = "log --graph --decorate --pretty=oneline --abbrev-commit --all";
            };
            commit.gpgsign = true;
            pull.rebase = true;
            rebase.autoStash = true;
            push.autoSetupRemote = true;
            format.signoff = true;
            status.submoduleSummary = false;
            tag.forceSignAnnotated = true;
            init.defaultBranch = "main";
            url."ssh://git@github.com/".insteadOf = "https://github.com/";
            merge.tool = lib.getExe pkgs.meld;
          };
        };
        lazygit.enable = true;
        fish.shellAbbrs.lg = lib.getExe config.programs.lazygit.package;
        difftastic = {
          enable = true;
          options.background = "dark";
          options.display = "inline";
          git = {
            enable = true;
            diffToolMode = true;
          };
        };
      };

      home.packages = [
        pkgs.meld
        pkgs.serie
      ];
    };
}
