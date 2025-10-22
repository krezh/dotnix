{ config, ... }:
{
  programs = {
    git = {
      enable = true;
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
      };
    };
    lazygit.enable = true;
    fish.shellAbbrs = {
      lg = "lazygit";
    };
  };
}
