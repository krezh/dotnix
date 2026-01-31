{ inputs, ... }:
{
  flake.modules.homeManager.browsers =
    {
      pkgs,
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
            "Kagi Assistant" = {
              id = "kagiassistant";
              url = "https://kagi.com/assistant";
              position = 107;
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
                urls = [ { template = "https://kagi.com/search?q={searchTerms}"; } ];
                SuggestURLTemplate = "https://kagi.com/api/autosuggest?q=%s";
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
            # Network settings
            "browser.preferences.defaultPerformanceSettings.enabled" = false;
            "network.http.max-connections" = 1200;
            "network.http.max-persistent-connections-per-server" = 8;
            "network.http.max-urgent-start-excessive-connections-per-host" = 5;
            "network.http.request.max-start-delay" = 5;
            "network.http.pacing.requests.enabled" = false;
            "network.http.pacing.requests.burst" = 32;
            "network.http.pacing.requests.min-parallelism" = 10;
            "network.ssl_tokens_cache_capacity" = 32768;
            "network.http.http3.enabled" = true;
            "network.http.speculative-parallel-limit" = 0;
            "network.dns.disablePrefetch" = true;
            "network.dns.disablePrefetchFromHTTPS" = true;
            "network.prefetch-next" = false;
            "network.predictor.enabled" = false;
            "network.predictor.enable-prefetch" = false;
            "browser.urlbar.speculativeConnect.enabled" = false;
            "browser.places.speculativeConnect.enabled" = false;

            # Memory settings
            "javascript.options.mem.high_water_mark" = 128;
            "browser.cache.disk.enable" = false;
            "browser.cache.disk.capacity" = 0;
            "browser.cache.memory.capacity" = 131072;
            "browser.cache.disk.smart_size.enabled" = false;
            "browser.cache.memory.max_entry_size" = 32768;
            "browser.cache.disk.metadata_memory_limit" = 16384;
            "browser.cache.max_shutdown_io_lag" = 100;
            "image.mem.max_decoded_image_kb" = 512000;
            "image.cache.size" = 10485760;
            "image.mem.decode_bytes_at_a_time" = 65536;
            "image.mem.shared.unmap.min_expiration_ms" = 90000;
            "media.memory_cache_max_size" = 1048576;
            "media.memory_caches_combined_limit_kb" = 4194304;
            "media.cache_readahead_limit" = 600;
            "media.cache_resume_threshold" = 300;

            # Session & storage
            "dom.storage.default_quota" = 20480;
            "dom.storage.shadow_writes" = true;
            "browser.sessionstore.interval" = 60000;
            "browser.sessionhistory.max_total_viewers" = 10;
            "browser.sessionstore.max_tabs_undo" = 10;
            "browser.sessionstore.max_entries" = 10;
            "browser.tabs.min_inactive_duration_before_unload" = 600000;

            # Content processing
            "content.maxtextrun" = 8191;
            "content.interrupt.parsing" = true;
            "content.notify.ontimer" = true;
            "content.notify.interval" = 50000;
            "content.max.tokenizing.time" = 2000000;
            "content.switch.threshold" = 300000;

            # Layout & rendering
            "layout.frame_rate" = -1;
            "nglayout.initialpaint.delay" = 5;
            "gfx.content.skia-font-cache-size" = 32;

            # GPU - WebRender
            "gfx.webrender.all" = true;
            "gfx.webrender.enabled" = true;
            "gfx.webrender.compositor" = true;
            "gfx.webrender.precache-shaders" = true;
            "gfx.webrender.software" = false;
            "gfx.webrender.layer-compositor" = true;
            "layers.acceleration.force-enabled" = true;
            "gfx.canvas.accelerated.cache-items" = 32768;
            "gfx.canvas.accelerated.cache-size" = 4096;
            "gfx.canvas.max-size" = 16384;
            "webgl.max-size" = 16384;
            "webgl.force-enabled" = true;
            "dom.webgpu.enabled" = true;

            # UI
            "ui.submenuDelay" = 0;
            "browser.uidensity" = 0;
            "dom.element.animate.enabled" = true;
            "general.smoothScroll" = true;
            "general.smoothScroll.msdPhysics.enabled" = false;
            "general.smoothScroll.currentVelocityWeighting" = 0;
            "general.smoothScroll.stopDecelerationWeighting" = 1;
            "general.smoothScroll.mouseWheel.durationMaxMS" = 150;
            "general.smoothScroll.mouseWheel.durationMinMS" = 50;
            "apz.overscroll.enabled" = false;
            "general.autoScroll" = true;

            # Process management
            "dom.ipc.processCount" = 8;
            "dom.ipc.keepProcessesAlive.web" = 4;
            "accessibility.force_disabled" = 1;
            "fission.autostart" = true;

            # Media
            "dom.media.webcodecs.h265.enabled" = true;
            "media.videocontrols.picture-in-picture.video-toggle.enabled" = false;
            "media.videocontrols.picture-in-picture.enable-when-switching-tabs.enabled" = false;
            "media.ffmpeg.vaapi.enabled" = true;
            "media.hardware-video-decoding.force-enabled" = true;
            "media.wmf.zero-copy-nv12-textures-force-enabled" = true;

            # Linux - Wayland
            "widget.wayland.opaque-region.enabled" = true;
            "widget.wayland.fractional-scale.enabled" = true;
            "widget.gtk.rounded-bottom-corners.enabled" = false;

            # Privacy
            "privacy.trackingprotection.enabled" = false;
            "privacy.query_stripping.enabled" = true;
            "privacy.query_stripping.enabled.pbmode" = true;
            "privacy.spoof_english" = 1;
            "privacy.firstparty.isolate" = true;
            "network.cookie.cookieBehavior" = 5;
            "dom.battery.enabled" = false;
            "network.http.referer.XOriginPolicy" = 0;
            "network.http.referer.XOriginTrimmingPolicy" = 0;
            "privacy.partition.network_state" = false;
            "browser.safebrowsing.downloads.remote.enabled" = false;

            # Disable AI & suggestions
            "browser.ml.chat.enabled" = false;
            "browser.search.suggest.enabled" = false;
            "browser.urlbar.suggest.searches" = false;
            "browser.findBar.suggest.enabled" = false;

            # Zen theme
            "zen.theme.accent-color" = "#ffffff90";
            "zen.theme.border-radius" = toString config.var.rounding;
            "zen.theme.content-element-separation" = 0;
            "zen.theme.gradient" = true;
            "zen.theme.gradient.show-custom-colors" = true;
            "zen.theme.acrylic-elements" = false;
            "zen.urlbar.replace-newtab" = true;
            "zen.urlbar.behavior" = "float";
            "zen.workspaces.open-new-tab-if-last-unpinned-tab-is-closed" = true;
            "zen.workspaces.continue-where-left-off" = true;
            "zen.workspaces.show-workspace-indicator" = false;
            "zen.splitView.enable-tab-drop" = false;
            "zen.tabs.show-newtab-vertical" = false;
            "zen.view.experimental-rounded-view" = false;
            "zen.view.gray-out-inactive-windows" = false;
            "zen.view.compact.enable-at-startup" = false;
            "zen.view.compact.hide-toolbar" = true;
            "zen.view.compact.toolbar-flash-popup" = true;
            "zen.view.show-newtab-button-top" = false;
            "zen.view.window.scheme" = 2;
            "zen.watermark.enabled" = false;
            "zen.mediacontrols.enabled" = false;
            "zen.welcome-screen.seen" = true;
            "reader.parse-on-load.enabled" = false;

            # Browser misc
            "browser.aboutConfig.showWarning" = false;
            "browser.tabs.warnOnClose" = false;
            "browser.tabs.hoverPreview.enabled" = true;
            "browser.newtabpage.activity-stream.feeds.topsites" = false;
            "browser.topsites.contile.enabled" = false;
            "browser.formfill.enable" = false;
            "browser.download.useDownloadDir" = true;
            "toolkit.legacyUserProfileCustomizations.stylesheets" = true;
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
    };
}
