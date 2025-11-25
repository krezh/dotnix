{
  pkgs,
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
  mkLockedAttrs = builtins.mapAttrs (
    _: value: {
      Value = value;
      Status = "locked";
    }
  );
in
{
  imports = [ inputs.zen-browser.homeModules.default ];
  programs.zen-browser = {
    enable = true;
    profiles.${config.home.username} = {
      isDefault = true;
      pinsForce = true;
      pins = {
        "ProtonMail" = {
          id = "protonmail";
          url = "https://mail.proton.me";
          position = 102;
          isEssential = true;
        };
        "Pushover" = {
          id = "pushover";
          url = "https://client.pushover.net/";
          position = 103;
          isEssential = true;
        };
        "AlertManager" = {
          id = "alertmanager";
          url = "https://alertmanager.talos.plexuz.xyz";
          position = 104;
          isEssential = true;
        };
        "Ntfy" = {
          id = "ntfy";
          url = "https://ntfy.plexuz.xyz";
          position = 105;
          isEssential = true;
        };
        "ChatGPT" = {
          id = "chatgpt";
          url = "https://chatgpt.com/";
          position = 106;
          isEssential = false;
        };
        "Claude" = {
          id = "claude";
          url = "https://claude.ai";
          position = 106;
          isEssential = false;
        };
      };
      search = {
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
              {
                template = "https://kagi.com/search?q={searchTerms}";
              }
            ];
            icon = "https://help.kagi.com/favicon-16x16.png";
            updateInterval = 24 * 60 * 60 * 1000; # every day
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
                ];
              }
            ];
            icon = "${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";
            definedAliases = [ "!np" ];
          };
          "NixOS Wiki" = {
            urls = [ { template = "https://wiki.nixos.org/index.php?search={searchTerms}"; } ];
            icon = "https://wiki.nixos.org/favicon.ico";
            updateInterval = 24 * 60 * 60 * 1000; # every day
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
                    value = "master"; # unstable
                  }
                ];
              }
            ];
            icon = "''${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";
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
        };
        order = [
          "Kagi"
          "Nix Packages"
          "NixOS Wiki"
          "Home Manager NixOs"
          "Github Code Search"
        ];
      };
      settings = {
        "browser.aboutConfig.showWarning" = false;
        "browser.tabs.warnOnClose" = false;
        "media.videocontrols.picture-in-picture.video-toggle.enabled" = true;

        "browser.tabs.hoverPreview.enabled" = true;
        "browser.newtabpage.activity-stream.feeds.topsites" = false;
        "browser.topsites.contile.enabled" = false;
        "browser.formfill.enable" = false;
        "browser.download.useDownloadDir" = true;

        "general.autoScroll" = true;

        # "permissions.default.desktop-notification" = 2; # 0=ask, 1=allow, 2=block

        "privacy.spoof_english" = 1;
        "zen.view.experimental-rounded-view" = false; # Fix Fonts rendering
        "privacy.firstparty.isolate" = true;
        "network.cookie.cookieBehavior" = 5;
        "dom.battery.enabled" = false;

        "gfx.webrender.all" = true;
        "network.http.http3.enabled" = true;
        "toolkit.legacyUserProfileCustomizations.stylesheets" = true; # enable userChrome.css

        "widget.gtk.rounded-bottom-corners.enabled" = false; # https://github.com/zen-browser/desktop/issues/6302

        "zen.splitView.enable-tab-drop" = false;
        "zen.workspaces.show-workspace-indicator" = false;
        "zen.view.gray-out-inactive-windows" = false;
        "zen.watermark.enabled" = false;
        "zen.theme.content-element-separation" = 0;
        "zen.tabs.show-newtab-vertical" = false;
        "zen.urlbar.behavior" = "float";
        "zen.view.compact.enable-at-startup" = false;
        "zen.view.compact.hide-toolbar" = true;
        "zen.view.compact.toolbar-flash-popup" = true;
        "zen.view.show-newtab-button-top" = false;
        "zen.view.window.scheme" = 2; # 0 dark theme, 1 light theme, 2 auto
        "zen.welcome-screen.seen" = true;
        "zen.workspaces.continue-where-left-off" = true;
        "zen.theme.border-radius" = 0;
        "zen.theme.gradient" = true;
        "zen.mediacontrols.enabled" = false;
      };
    };
    policies = {
      Preferences = mkLockedAttrs { };
      AutofillAddressEnabled = true;
      AutofillCreditCardEnabled = false;
      DisableAppUpdate = true;
      DisableFeedbackCommands = true;
      DisableFirefoxStudies = true;
      DisablePocket = true;
      DisableTelemetry = true;
      DontCheckDefaultBrowser = true;
      NoDefaultBookmarks = true;
      OfferToSaveLogins = false;
      EnableTrackingProtection = {
        Value = true;
        Locked = true;
        Cryptomining = true;
        Fingerprinting = true;
      };
    };
  };

  home.file.".zen/${config.home.username}/chrome" = catppuccin;
}
