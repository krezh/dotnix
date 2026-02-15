{
  flake.modules.homeManager.krezh =
    { config, pkgs, ... }:
    {
      programs.jujutsu = {
        enable = true;
        settings = {
          user = {
            name = "Krezh";
            email = "krezh@users.noreply.github.com";
          };
          ui = {
            editor = "nvim";
            paginate = "never";
            merge-editor = ":builtin";
            default-command = [
              "log"
              "--reversed"
            ];
          };
          signing = {
            behavior = "own";
            backend = "ssh";
            key = config.sops.secrets."ssh/privkey".path;
          };
        };
      };

      programs.git = {
        enable = true;
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
          gpg.format = "ssh";
          format.signoff = true;
          status.submoduleSummary = false;
          tag.forceSignAnnotated = true;
          init.defaultBranch = "main";
          url."ssh://git@github.com/".pushInsteadOf = "https://github.com/";
          merge.tool = "meld";
        };
      };

      programs.lazygit.enable = true;

      programs.fish.shellAbbrs.lg = "lazygit";

      home.packages = [
        pkgs.meld
        pkgs.git
      ];
    };
}
