{
  flake.modules.homeManager.shell =
    {
      lib,
      config,
      pkgs,
      ...
    }:
    let
      condDef =
        name:
        let
          packageNames = map (p: p.pname or p.name or null) config.home.packages;
          hasPackage = lib.elem name packageNames;
        in
        lib.mkIf hasPackage name;
    in
    {
      # Fish shell
      programs.fish = {
        enable = true;
        package = pkgs.fish;
        shellAbbrs = {
          gs = "git status";
          gc = "git commit";
          gcm = "git ci -m";
          gco = "git co";
          ga = "git add -A";
          gm = "git merge";
          gl = "git l";
          gd = "git diff";
          gb = "git b";
          gpl = "git pull";
          gp = "git push";
          gpc = "git push -u origin (git rev-parse --abbrev-ref HEAD)";
          gpf = "git push --force-with-lease";
          gbc = "git nb";
          curl = condDef "curlie";
        };
        shellAliases = {
          clear = "printf '\\033[2J\\033[3J\\033[1;1H'";
        };
        plugins = [
          {
            name = "puffer";
            inherit (pkgs.fishPlugins.puffer) src;
          }
          {
            name = "autopair";
            inherit (pkgs.fishPlugins.autopair) src;
          }
          {
            name = "bass";
            inherit (pkgs.fishPlugins.bass) src;
          }
        ];
        functions.fish_greeting = "";
        interactiveShellInit = ''
          ${lib.getExe pkgs.fastfetch}
          ${lib.getExe pkgs.any-nix-shell} fish --info-right | source

          function _ssh_known_host_hint --on-event fish_postexec
            set -l last_status $status
            if not string match -qr '^ssh\b' -- $argv[1]
              return
            end
            if test $last_status -ne 0
              set -l host (string replace -r '.*\s+' "" -- $argv[1] | string replace -r '^[^@]+@' "")
              printf '\nIf the host key changed, remove it with:\n  ssh-keygen -R "%s"\n' $host
            end
          end
        '';
      };
    };

  flake.modules.nixos.shell = {
    # Fish shell
    programs.fish = {
      enable = true;
      vendor = {
        completions.enable = true;
        config.enable = true;
        functions.enable = true;
      };
    };
  };
}
