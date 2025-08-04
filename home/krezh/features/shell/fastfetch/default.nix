{
  lib,
  pkgs,
  ...
}:
{
  programs.fastfetch = {
    enable = true;
    package = pkgs.fastfetch;
    settings = {
      "$schema" = "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json";
      logo = "none";
      display = {
        color = {
          keys = "white";
          title = "white";
        };
        percent = {
          type = 9;
        };
        separator = " ";
      };
      modules = [
        {
          type = "custom";
          key = "╭───────────╮";
          keyColor = "white";
        }
        {
          type = "title";
          key = "│ {#yellow}{#}  user   │";
          keyColor = "white";
          color = {
            user = "red";
            host = "yellow";
          };
        }
      ]
      ++ lib.optionals pkgs.stdenv.isLinux [
        {
          type = "os";
          key = "│ {#green}󰻀{#}  distro │";
          format = "{#green}{3}{#cyan}";
          keyColor = "white";
        }
        {
          type = "kernel";
          key = "│ {#cyan}󰌢 {#} kernel │";
          format = "{#cyan}{2}{#red}";
          keyColor = "white";
        }
        {
          type = "uptime";
          key = "│ {#blue} {#} uptime │";
          format = "{#blue}{?1}{1}d {?}{?2}{2}h {?}{?3}{3}m {?}{?4}{4}s{?}{#}";
          keyColor = "white";
        }
        {
          type = "packages";
          key = "│ {#red}󰏖{#}  pkgs   │";
          format = "{#red}{9}(sys) {10}(user){#}";
          keyColor = "white";
        }
      ]
      ++ [
        {
          type = "shell";
          key = "│ {#magenta} {#} shell  │";
          format = "{#magenta}{}{#}";
          keyColor = "white";
        }
        {
          type = "memory";
          key = "│ {#yellow}󰍛{#}  memory │";
          format = "{#yellow}{1} | {2}{#}";
          keyColor = "white";
        }
        {
          type = "custom";
          key = "╰───────────╯";
          keyColor = "white";
        }
        "break"
      ];
    };
  };
}
