{
  config,
  ...
}:
{
  sops = {
    age.keyFile = "${config.xdg.configHome}/sops/age/keys.txt";
    defaultSopsFile = ./secrets.sops.yaml;
    secrets = {
      "ssh/privkey" = {
        path = "/home/${config.home.username}/.ssh/id_ed25519";
        mode = "0600";
      };
      "atuin/key" = {
        path = "${config.xdg.configHome}/atuin/key";
      };
      "zipline/token" = {
        path = "${config.xdg.configHome}/zipline/token";
      };
      "kopia/password" = {
        path = "${config.xdg.configHome}/kopia/repository.password";
      };
    };
  };
  home = {
    sessionVariables = {
      SOPS_AGE_KEY_FILE = "${config.sops.age.keyFile}";
      SOPS_AGE_KEY_CMD = "age-plugin-yubikey --identity";
    };
  };
}
