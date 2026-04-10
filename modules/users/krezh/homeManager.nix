{ inputs, ... }:
let
  username = "krezh";
in
{
  flake.modules.homeManager.${username} =
    { config, ... }:
    {
      imports = with inputs.self.modules.homeManager; [
        atuin
        fastfetch
        aria2
        television
        superfile
      ];
      home = {
        username = "${username}";
        sessionVariables = {
          FLAKE = "${config.home.homeDirectory}/dotnix";
          NH_FLAKE = "${config.home.homeDirectory}/dotnix";
          SOPS_AGE_KEY_FILE = "${config.sops.age.keyFile}";
        };
      };
    };
}
