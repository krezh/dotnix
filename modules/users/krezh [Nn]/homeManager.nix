{ inputs, ... }:
let
  username = "krezh";
in
{
  flake.modules.homeManager.${username} =
    { config, ... }:
    {
      home = {
        username = "${username}";
        sessionVariables = {
          FLAKE = "${config.home.homeDirectory}/nix-config";
          NH_FLAKE = "${config.home.homeDirectory}/nix-config";
          SOPS_AGE_KEY_FILE = "${config.sops.age.keyFile}";
        };
      };
      imports = with inputs.self.modules.homeManager; [
        kubernetes
        atuin
        fastfetch
        aria2
        television
        superfile
      ];
    };
}
