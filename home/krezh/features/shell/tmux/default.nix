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
    escapeTime = 0;
  };
  programs.fzf.tmux.enableShellIntegration = true;
}
