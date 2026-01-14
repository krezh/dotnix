{
  flake.modules.homeManager.git =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      cfg = config.programs.git-config;
    in
    {
      options.programs.git-config = {
        enable = lib.mkEnableOption "git configuration";

        userName = lib.mkOption {
          type = lib.types.str;
          description = "Git user name";
        };

        userEmail = lib.mkOption {
          type = lib.types.str;
          description = "Git user email";
        };

        signingKeyPath = lib.mkOption {
          type = lib.types.str;
          description = "Path to the SSH signing key";
        };
      };

      config = lib.mkIf cfg.enable {
        programs.git = {
          enable = true;
          settings = {
            user = {
              name = cfg.userName;
              email = cfg.userEmail;
              signingkey = cfg.signingKeyPath;
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
    };
}
