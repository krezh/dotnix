{
  config,
  ...
}:
{
  programs.floorp = {
    enable = true;
    policies = {
      OfferToSaveLogins = false;
    };
    profiles.${config.home.username} = {
      isDefault = true;
      extensions.force = true;
      search.force = true;
      search.default = "google";
      search.engines = {
        wikipedia.hidden = true;
        ecosia.metaData.hidden = true;
        "Github Code Search" = {
          definedAliases = [ "gh" ];
          urls = [
            {
              template = "https://github.com/search";
              params = [
                {
                  name = "q";
                  value = "{searchTerms}";
                }
                {
                  name = "type";
                  value = "Code";
                }
              ];
            }
          ];
        };
        "Searchix" = {
          definedAliases = [ "si" ];
          urls = [
            {
              template = "https://searchix.ovh";
              params = [
                {
                  name = "query";
                  value = "{searchTerms}";
                }
              ];
            }
          ];
        };
        "Home Manager" = {
          definedAliases = [ "hm" ];
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
        };
        "Nix Packages" = {
          definedAliases = [ "np" ];
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
        };
        "Nix Options" = {
          definedAliases = [ "no" ];
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
        };
      };
    };
  };
}
