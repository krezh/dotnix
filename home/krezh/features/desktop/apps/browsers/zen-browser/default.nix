{
  inputs,
  config,
  ...
}:
let
  catppuccin = {
    source = "${inputs.zen-browser-catppuccin}/themes/Mocha/Lavender/";
    recursive = true;
    force = true;
  };
in
{
  imports = [ inputs.zen-browser.homeModules.default ];

  programs.zen-browser = {
    enable = true;
    profiles.${config.home.username} = {
      isDefault = true;
      extraConfig = ''
        ${builtins.readFile "${inputs.betterfox}/zen/user.js"}
      '';
    };
    policies = {
      OfferToSaveLogins = false;
    };
  };

  home.file.".zen/${config.home.username}/chrome" = catppuccin;
}
