{
  flake.modules.homeManager.krezh =
    { config, ... }:
    {
      sops = {
        age.keyFile = "${config.xdg.configHome}/sops/age/keys.txt";
        defaultSopsFile = ./secrets.sops.yaml;
        secrets = {
          "ssh/privkey" = {
            path = "${config.home.homeDirectory}/.ssh/id_ed25519";
            mode = "0600";
          };
          "atuin/key".path = "${config.xdg.configHome}/atuin/key";
          "zipline/token".path = "${config.xdg.configHome}/zipline/token";
          "kopia/password".path = "${config.xdg.configHome}/kopia/repository.password";
          "github/mcp_token".path = "${config.xdg.configHome}/github/mcp_token";
          "garage/accessID" = { };
          "garage/accessSecret" = { };
        };
      };
      home.sessionVariables = {
        SOPS_AGE_KEY_FILE = "${config.sops.age.keyFile}";
      };
    };
}
