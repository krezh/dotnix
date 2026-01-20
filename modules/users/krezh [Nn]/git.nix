{inputs, ...}: {
  flake.modules.homeManager.krezh = {config, ...}: {
    imports = [inputs.self.modules.homeManager.git];
    programs.git-config = {
      enable = true;
      userName = "Krezh";
      userEmail = "krezh@users.noreply.github.com";
      signingKeyPath = config.sops.secrets."ssh/privkey".path;
    };
  };
}
