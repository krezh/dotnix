{ config, ... }: {
  programs.git = {
    enable = true;
    userName = "Krezh";
    userEmail = "krezh@users.noreply.github.com";
    extraConfig = {
      commit.gpgsign = true;
      pull.rebase = true;
      push.autoSetupRemote = true;
      gpg.format = "ssh";
      format.signoff = true;
      status.submoduleSummary = false;
      tag.forceSignAnnotated = true;
      user.signingkey = config.sops.secrets."ssh/privkey".path;
      init.defaultBranch = "main";
      url."ssh://git@github.com/".pushInsteadOf = "https://github.com/";
    };
  };
}
