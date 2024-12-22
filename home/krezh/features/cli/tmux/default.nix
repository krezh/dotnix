{ ... }:
{
  programs.tmux = {
    enable = true;
    clock24 = true;
    newSession = true;
    mouse = true;
    prefix = "C-a";
    keyMode = "vi";
    baseIndex = 1;
    terminal = "screen-256color";
  };
  programs.fzf.tmux.enableShellIntegration = true;
}
