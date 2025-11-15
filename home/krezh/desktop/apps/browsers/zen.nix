{ inputs, config, ... }:
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
      extraConfig = ''
        ${builtins.readFile "${inputs.betterfox}/zen/user.js"}
      '';
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

      };
      search.force = true;
      search.default = "Kagi";
      search.engines = {
        wikipedia.hidden = true;
        ecosia.metaData.hidden = true;
        Kagi = {
          definedAliases = [ "!k" ];
          icon = "https://kagi.com/asset/80be37f/favicon-32x32.png?v=f6272da653e54e2a660a4d2bd696947a903fb130";
          updateInterval = 24 * 60 * 60 * 1000;
          urls = [
            { template = "https://kagi.com/search?q={searchTerms}"; }
            {
              type = "application/x-suggestions+json";
              template = "https://kagi.com/api/autosuggest?q={searchTerms}";
            }
          ];
        };
        "GithubCodeSearch" = {
          urls = [ { template = "https://github.com/search?{searchTerms}&type=code"; } ];
          definedAliases = [ "!gh" ];
        };
        "Searchix" = {
          urls = [ { template = "https://searchix.ovh?{searchTerms}"; } ];
          definedAliases = [ "!si" ];
        };
        "HomeManager" = {
          urls = [
            { template = "https://home-manager-options.extranix.com?q={searchTerms}&release=master"; }
          ];
          definedAliases = [ "!hm" ];
        };
        "NixPackages" = {
          urls = [
            {
              template = "https://search.nixos.org/packages?channel=unstable&query={searchTerms}&type=packages";
            }
          ];
          definedAliases = [ "!np" ];
        };
        "NixOptions" = {
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
          definedAliases = [ "!no" ];
        };
      };
      settings = {
        "browser.aboutConfig.showWarning" = false;
        "browser.tabs.warnOnClose" = false;
        "media.videocontrols.picture-in-picture.video-toggle.enabled" = true;

        # Disable swipe gestures (Browser:BackOrBackDuplicate, Browser:ForwardOrForwardDuplicate)
        "browser.gesture.swipe.left" = "";
        "browser.gesture.swipe.right" = "";
        "browser.tabs.hoverPreview.enabled" = true;
        "browser.newtabpage.activity-stream.feeds.topsites" = false;
        "browser.topsites.contile.enabled" = false;
        "browser.formfill.enable" = false;

        "browser.search.defaultenginename" = "Kagi";
        "browser.search.order.1" = "Kagi";

        "privacy.resistFingerprinting" = false;
        "privacy.resistFingerprinting.randomization.canvas.use_siphash" = true;
        "privacy.resistFingerprinting.randomization.daily_reset.enabled" = true;
        "privacy.resistFingerprinting.randomization.daily_reset.private.enabled" = true;
        "privacy.resistFingerprinting.block_mozAddonManager" = true;
        "privacy.spoof_english" = 1;
        "zen.view.experimental-rounded-view" = false; # Fix Fonts rendering
        "privacy.firstparty.isolate" = true;
        "network.cookie.cookieBehavior" = 5;
        "dom.battery.enabled" = false;

        "gfx.webrender.all" = true;
        "network.http.http3.enabled" = true;
        "network.socket.ip_addr_any.disabled" = true; # disallow bind to 0.0.0.0
        "toolkit.legacyUserProfileCustomizations.stylesheets" = true; # enable userChrome.css

        "widget.gtk.rounded-bottom-corners.enabled" = false; # https://github.com/zen-browser/desktop/issues/6302
      };
    };
    policies = {
      Preferences = mkLockedAttrs {
        "zen.theme.content-element-separation" = 0;
        "zen.tabs.show-newtab-vertical" = false;
        "zen.urlbar.behavior" = "float";
        "zen.view.compact.enable-at-startup" = false;
        "zen.view.compact.hide-toolbar" = true;
        "zen.view.compact.toolbar-flash-popup" = true;
        "zen.view.show-newtab-button-top" = false;
        "zen.view.window.scheme" = 0;
        "zen.welcome-screen.seen" = true;
        "zen.workspaces.continue-where-left-off" = true;
        "zen.theme.border-radius" = 0;
        "zen.theme.gradient" = false;
        "zen.mediacontrols.enabled" = false;
      };
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
