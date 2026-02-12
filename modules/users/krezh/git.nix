{
  flake.modules.homeManager.krezh =
    { config, ... }:
    {
      programs.git-config = {
        enable = true;
        userName = "Krezh";
        userEmail = "krezh@users.noreply.github.com";
        signingKeyPath = config.sops.secrets."ssh/privkey".path;
      };
    };
}
