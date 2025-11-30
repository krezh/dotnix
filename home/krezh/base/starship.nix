{ pkgs, lib, ... }:
{
  programs.starship = {
    enable = true;
    enableFishIntegration = true;
    enableBashIntegration = true;
    enableZshIntegration = true;
    settings = {
      add_newline = true;
      format = "$username$hostname$git_branch$git_commit$git_state$git_metrics$git_status$fill$cmd_duration$time\n$directory$fill$kubernetes\${custom.talos}\n$character";
      kubernetes = {
        format = "[$context](bold blue) $symbol ";
        symbol = "⎈";
        disabled = false;
        contexts = [
          {
            context_pattern = "^(?<url>[^-]+)-(?<cluster>.+)$";
            symbol = "⎈";
            context_alias = "$cluster";
          }
        ];
      };
      custom.talos = {
        command = "${lib.getExe pkgs.talosctl} config info --output json | ${lib.getExe pkgs.jq} --raw-output '.context' | tr -d '\\n'";
        format = "[$output](bold blue)";
        when = "command -v ${lib.getExe pkgs.talosctl} &>/dev/null";
        disabled = false;
      };
      fill = {
        symbol = " ";
      };
      direnv = {
        disabled = false;
      };
      time = {
        disabled = false;
        style = "bold bright-black";
        format = "[$time]($style)";
      };
      nix_shell = {
        disabled = false;
        impure_msg = "[$name](bold red)";
        pure_msg = "[$name](bold green)";
        unknown_msg = "[$name](bold yellow)";
        format = "[$state](bold blue) ";
        heuristic = true;
      };
      cmd_duration = {
        format = "took [$duration]($style) ";
        style = "yellow bold";
        show_notifications = true;
        min_time_to_notify = 60000;
      };
      username = {
        style_user = "green bold";
        style_root = "red bold";
        format = "[$user]($style)";
        disabled = false;
        show_always = true;
      };
      hostname = {
        ssh_only = false;
        format = "@[$hostname](blue bold) ";
        disabled = false;
      };
      jobs = {
        symbol = " ";
        format = "[$number$symbol]($style) ";
        style = "bold blue";
      };
      sudo = {
        format = "[$symbol ]()";
        symbol = "💀";
        disabled = true;
      };
      container = {
        disabled = true;
      };
      git_branch = {
        symbol = " ";
        format = "[$symbol$branch(:$remote_branch)]($style) ";
      };
      character = {
        success_symbol = "[❯](bold green)";
        error_symbol = "[❯](bold red)";
      };
    };
  };
}
