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
      search.force = true;
      search.default = "google";
      search.engines = {
        wikipedia.hidden = true;
        ecosia.metaData.hidden = true;
        "Home Manager" = {
          urls = [
            {
              template = "https://home-manager-options.extranix.com/";
              params = [
                {
                  name = "query";
                  value = "{searchTerms}";
                }
                {
                  name = "release";
                  value = "master";
                }
              ];
            }
          ];
          definedAliases = [ "hm" ];
        };
        "Nix Packages" = {
          urls = [
            {
              template = "https://search.nixos.org/packages";
              params = [
                {
                  name = "channel";
                  value = "unstable";
                }
                {
                  name = "type";
                  value = "packages";
                }
                {
                  name = "query";
                  value = "{searchTerms}";
                }
              ];
            }
          ];
          definedAliases = [ "np" ];
        };
        "Nix Options" = {
          urls = [
            {
              template = "https://search.nixos.org/options";
              params = [
                {
                  name = "channel";
                  value = "unstable";
                }
                {
                  name = "type";
                  value = "packages";
                }
                {
                  name = "query";
                  value = "{searchTerms}";
                }
              ];
            }
          ];
          definedAliases = [ "no" ];
        };
      };
    };
    policies = {
      OfferToSaveLogins = false;
    };
  };

  home.file.".zen/${config.home.username}/chrome" = catppuccin;
}
