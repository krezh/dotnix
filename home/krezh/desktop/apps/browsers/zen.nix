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

        "permissions.default.desktop-notification" = 0; # 0=ask, 1=allow, 2=block

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

        # fastfox
        "gfx.content.skia-font-cache-size" = 32;
        "gfx.canvas.accelerated.cache-items" = 32768;
        "gfx.canvas.accelerated.cache-size" = 4096;
        "webgl.max-size" = 16384;
        "browser.cache.disk.enable" = false;
        "browser.cache.memory.capacity" = 131072;
        "browser.cache.memory.max_entry_size" = 20480;
        "browser.sessionhistory.max_total_viewers" = 4;
        "browser.sessionstore.max_tabs_undo" = 10;
        "media.memory_cache_max_size" = 262144;
        "media.memory_caches_combined_limit_kb" = 1048576;
        "media.cache_readahead_limit" = 600;
        "media.cache_resume_threshold" = 300;
        "image.cache.size" = 10485760;
        "image.mem.decode_bytes_at_a_time" = 65536;
        "network.http.max-connections" = 1800;
        "network.http.max-persistent-connections-per-server" = 10;
        "network.http.request.max-start-delay" = 5;
        "network.http.pacing.requests.enabled" = false;
        "network.dnsCacheEntries" = 10000;
        "network.dnsCacheExpiration" = 3600;
        "network.ssl_tokens_cache_capacity" = 10240;
        "network.http.speculative-parallel-limit" = 0;
        "network.dns.disablePrefetch" = true;
        "network.dns.disablePrefetchFromHTTPS" = true;
        "browser.urlbar.speculativeConnect.enabled" = false;
        "browser.places.speculativeConnect.enabled" = false;
        "network.prefetch-next" = false;
        "network.predictor.enabled" = false;

        # securefox
        "browser.contentblocking.category" = "strict";
        "privacy.trackingprotection.allow_list.baseline.enabled" = true;
        "privacy.trackingprotection.allow_list.convenience.enabled" = true;
        "security.OCSP.enabled" = 0;
        "security.pki.crlite_mode" = 2;
        "browser.sessionstore.interval" = 60000;
        "signon.formlessCapture.enabled" = false;
        "signon.privateBrowsingCapture.enabled" = false;
        "network.auth.subresource-http-auth-allow" = 1;
        "editor.truncate_user_pastes" = false;
        "network.http.referer.XOriginTrimmingPolicy" = 2;
        "permissions.default.geo" = 2;
        "geo.provider.network.url" = "https://beacondb.net/v1/geolocate";
        "browser.search.update" = false;
        "permissions.manager.defaultsUrl" = "";
        "browser.newtabpage.activity-stream.default.sites" = true;
        "dom.text_fragments.create_text_fragment.enabled" = true;

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
