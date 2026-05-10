_: {
  flake.modules.homeManager.browsers =
    { pkgs, config, ... }:
    {
      programs.zen-browser.profiles.${config.home.username}.search = {
        force = true;
        default = "Kagi";
        privateDefault = "Kagi";
        engines = {
          bing.metaData.hidden = true;
          google.metaData.hidden = true;
          ddg.metaData.hidden = true;
          wikipedia.metaData.hidden = true;
          perplexity.metaData.hidden = true;
          "Kagi" = {
            urls = [
              { template = "https://kagi.com/search?q={searchTerms}"; }
              {
                type = "application/x-suggestions+json";
                template = "https://kagi.com/api/autosuggest?q={searchTerms}";
              }
            ];
            icon = "https://help.kagi.com/favicon-16x16.png";
            updateInterval = 24 * 60 * 60 * 1000;
            definedAliases = [ "!kg" ];
          };
          "Nix Packages" = {
            urls = [
              {
                template = "https://search.nixos.org/packages";
                params = [
                  {
                    name = "type";
                    value = "packages";
                  }
                  {
                    name = "query";
                    value = "{searchTerms}";
                  }
                  {
                    name = "channel";
                    value = "unstable";
                  }
                ];
              }
            ];
            icon = "${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";
            definedAliases = [ "!np" ];
          };
          "NixOS Wiki" = {
            urls = [ { template = "https://wiki.nixos.org/index.php?search={searchTerms}"; } ];
            icon = "https://wiki.nixos.org/favicon.ico";
            updateInterval = 24 * 60 * 60 * 1000;
            definedAliases = [ "!nw" ];
          };
          "Home Manager NixOs" = {
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
            icon = "${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";
            definedAliases = [ "!hm" ];
          };
          "Github Code Search" = {
            urls = [
              {
                template = "https://github.com/search?q&type=code";
                params = [
                  {
                    name = "q";
                    value = "{searchTerms}";
                  }
                ];
              }
            ];
            definedAliases = [ "!gh" ];
          };
          "ProtonDB" = {
            urls = [
              {
                template = "https://www.protondb.com/search";
                params = [
                  {
                    name = "q";
                    value = "{searchTerms}";
                  }
                ];
              }
            ];
            definedAliases = [ "!pd" ];
          };
        };
        order = [
          "Kagi"
          "Nix Packages"
          "NixOS Wiki"
          "Home Manager NixOs"
          "Github Code Search"
        ];
      };
    };
}
