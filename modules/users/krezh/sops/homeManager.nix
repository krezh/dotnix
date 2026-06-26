{
  flake.modules.homeManager.krezh =
    { config, pkgs, ... }:
    {
      sops = {
        age = {
          keyFile = "${config.xdg.configHome}/sops/age/keys.txt";
          plugins = [
            pkgs.age-plugin-fido2-hmac
            pkgs.age-plugin-yubikey
          ];
        };
        defaultSopsFile = ./secrets.sops.yaml;
        secrets = {
          "ssh/privkey".path = "${config.home.homeDirectory}/.ssh/id_ed25519";
          "atuin/key".path = "${config.xdg.dataHome}/atuin/key";
          "zipline/token".path = "${config.xdg.configHome}/zipline/token";
          "kopia/password".path = "${config.xdg.configHome}/kopia/repository.password";
          "garage/accessID" = { };
          "garage/accessSecret" = { };
          "r2/accessID" = { };
          "r2/accessSecret" = { };
          "hetzner/accessID" = { };
          "hetzner/accessSecret" = { };
        };
      };
      home.sessionVariables = {
        SOPS_AGE_KEY_FILE = "${config.sops.age.keyFile}";
      };
    };
}
