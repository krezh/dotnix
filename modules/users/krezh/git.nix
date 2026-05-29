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
              ai-commit = ''!diff=$(git diff --cached); if [ -z "$diff" ]; then echo "ai-commit: nothing staged" >&2; exit 1; fi; msg=$(printf '%s' "$diff" | ${lib.getExe config.programs.claude-code.package} --model haiku -p 'Write a single-line Conventional Commits message in imperative mood summarizing this diff. Output ONLY the message text, with no surrounding quotes, backticks, or explanation.'); git commit -m "$msg"'';
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
        jujutsu = {
          enable = true;
          settings = {
            user = {
              name = "Krezh";
              email = "krezh@users.noreply.github.com";
            };
            signing = {
              behavior = "own";
              backend = "ssh";
              key = config.sops.secrets."ssh/privkey".path;
            };
            ui = {
              default-command = "log";
              diff-formatter = [
                (lib.getExe pkgs.difftastic)
                "--color=always"
                "$left"
                "$right"
              ];
              pager = "less -FRX";
            };
            git.push-bookmark-prefix = "krezh/push-";
            git.colocate = true;
            git.track-default-bookmark-on-clone = true;
            aliases = {
              bs = [
                "bookmark"
                "set"
              ];
              sum = [
                "show"
                "@"
                "--summary"
              ];
              tug = [
                "bookmark"
                "move"
                "--from"
                "closest_bookmarks(@)"
                "--to"
                "@"
              ];
              ai-desc = [
                "util"
                "exec"
                "--"
                (lib.getExe pkgs.bash)
                "-c"
                ''
                  set -euo pipefail
                  diff=$(${lib.getExe config.programs.jujutsu.package} diff --ignore-working-copy --git)
                  if [ -z "$diff" ]; then
                    echo "desc-ai: no changes in @ to describe" >&2
                    exit 1
                  fi
                  msg=$(printf '%s\n' "$diff" | ${lib.getExe config.programs.claude-code.package} --model haiku -p 'Write a single-line Conventional Commits message in imperative mood summarizing this diff. Output ONLY the message text, with no surrounding quotes, backticks, or explanation.')
                  ${lib.getExe config.programs.jujutsu.package} describe --ignore-working-copy -m "$msg"
                ''
              ];
              pull = [
                "util"
                "exec"
                "--"
                (lib.getExe pkgs.bash)
                "-c"
                ''
                  set -euo pipefail
                  jj=${lib.getExe config.programs.jujutsu.package}
                  "$jj" git fetch
                  "$jj" rebase -d 'trunk()'
                ''
              ];
            };
            revset-aliases."closest_bookmarks(to)" = "heads(::to & bookmarks())";
            templates = {
              draft_commit_description = ''
                concat(
                  builtin_draft_commit_description,
                  "\nJJ: ignore-rest\n",
                  diff.git(),
                )
              '';
              new_description = ''
                if(parents.len() > 1,
                  "Merge " ++ parents.skip(1).map(|p| if(
                    p.bookmarks(),
                    p.bookmarks().first().name(),
                    p.change_id().shortest(8)
                  )).join(", ") ++ " into " ++ if(
                    parents.first().bookmarks(),
                    parents.first().bookmarks().first().name(),
                    parents.first().change_id().shortest(8)
                  ) ++ "\n",
                  ""
                )
              '';
            };
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
        pkgs.jjui
        pkgs.lazyjj
      ];
    };
}
