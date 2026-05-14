{ inputs, ... }:
{
  flake.modules.homeManager.browsers =
    { pkgs, config, ... }:
    let
      # Patch catppuccin to remove element separation
      catppuccinPatched = pkgs.runCommand "catppuccin-zen-patched" { } ''
        cp -r ${inputs.zen-browser-catppuccin}/themes/Mocha/Blue $out
        chmod -R +w $out

        # Remove element separation and webview border radius
        sed -i 's/--zen-element-separation: 8px/--zen-element-separation: 0px/g' $out/userChrome.css
        sed -i 's/--zen-webview-border-radius: 10px/--zen-webview-border-radius: 0px/g' $out/userChrome.css
      '';

      catppuccin = {
        source = catppuccinPatched;
        recursive = true;
        force = true;
      };

      lock = Value: {
        inherit Value;
        Status = "locked";
      };
    in
    {
      imports = [ inputs.zen-browser.homeModules.twilight-official ];

      programs.zen-browser = {
        enable = true;
        setAsDefaultBrowser = true;
        profiles.${config.home.username} = {
          isDefault = true;
          mods = [ ];
          settings = {
            # https://docs.zen-browser.app/guides/about-config-flags
            # Memory settings
            "javascript.options.mem.high_water_mark" = 128;

            # Content processing
            "content.maxtextrun" = 8191;
            "content.interrupt.parsing" = true;
            "content.notify.ontimer" = true;
            "content.notify.interval" = 50000;
            "content.max.tokenizing.time" = 2000000;
            "content.switch.threshold" = 300000;

            # Layout & rendering
            "nglayout.initialpaint.delay" = 5;

            # UI
            "general.smoothScroll.msdPhysics.enabled" = false;
            "general.smoothScroll.currentVelocityWeighting" = 0;
            "general.smoothScroll.stopDecelerationWeighting" = 1;
            "general.smoothScroll.mouseWheel.durationMaxMS" = 150;
            "general.smoothScroll.mouseWheel.durationMinMS" = 50;
            "apz.overscroll.enabled" = false;

            # Process management
            "fission.autostart" = true;

            # Privacy
            "privacy.query_stripping.enabled" = true;
            "privacy.query_stripping.enabled.pbmode" = true;
            "privacy.spoof_english" = 1;
            "privacy.firstparty.isolate" = false;
            "privacy.partition.network_state" = false;

            # Zen theme
            "zen.theme.accent-color" = "#ffffff90";
            "zen.theme.border-radius" = toString config.var.rounding;
            "zen.theme.content-element-separation" = 0;
            "zen.theme.gradient" = false;
            "zen.theme.gradient.show-custom-colors" = true;
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
            "zen.welcome-screen.seen" = true;
            "zen.watermark.enabled" = false;
            "zen.mediacontrols.enabled" = false;
            "zen.window-sync.enabled" = false;
            "zen.window-sync.sync-only-pinned-tabs" = true;
            "zen.view.use-native-titlebar" = true;
            "reader.parse-on-load.enabled" = false;
            "zen.tabs.dnd-open-blank-window" = false;

            # Transparancy
            "widget.transparent-windows" = true;
          };
        };
        policies = {
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
          SearchSuggestEnabled = true;
          DisableFormHistory = true;
          PromptForDownloadLocation = false;
          EnableTrackingProtection = {
            Value = true;
            Locked = true;
            Cryptomining = true;
            Fingerprinting = true;
          };
          FirefoxHome = {
            TopSites = false;
            Highlights = false;
            Pocket = false;
            Snippets = false;
            Locked = true;
          };
          Preferences = {
            # Network settings
            "network.http.max-connections" = lock 1200;
            "network.http.max-persistent-connections-per-server" = lock 8;
            "network.http.max-urgent-start-excessive-connections-per-host" = lock 5;
            "network.http.request.max-start-delay" = lock 5;
            "network.http.pacing.requests.enabled" = lock false;
            "network.http.pacing.requests.burst" = lock 32;
            "network.http.pacing.requests.min-parallelism" = lock 10;
            "network.ssl_tokens_cache_capacity" = lock 32768;
            "network.http.http3.enabled" = lock true;
            "network.dns.disablePrefetch" = lock true;
            "network.dns.disablePrefetchFromHTTPS" = lock true;
            "network.prefetch-next" = lock false;
            "network.predictor.enabled" = lock false;
            "network.predictor.enable-prefetch" = lock false;
            "network.http.speculative-parallel-limit" = lock 0;
            "network.cookie.cookieBehavior" = lock 5;
            "network.http.referer.XOriginPolicy" = lock 0;
            "network.http.referer.XOriginTrimmingPolicy" = lock 0;

            # Browser & Profile settings
            "browser.preferences.defaultPerformanceSettings.enabled" = lock false;
            "browser.cache.disk.enable" = lock false;
            "browser.cache.disk.capacity" = lock 0;
            "browser.cache.memory.capacity" = lock 131072;
            "browser.cache.disk.smart_size.enabled" = lock false;
            "browser.cache.memory.max_entry_size" = lock 32768;
            "browser.cache.disk.metadata_memory_limit" = lock 16384;
            "browser.cache.max_shutdown_io_lag" = lock 100;
            "browser.sessionstore.interval" = lock 60000;
            "browser.sessionhistory.max_total_viewers" = lock 10;
            "browser.sessionstore.max_tabs_undo" = lock 10;
            "browser.sessionstore.max_entries" = lock 10;
            "browser.tabs.min_inactive_duration_before_unload" = lock 600000;
            "browser.uidensity" = lock 0;
            "browser.ml.chat.enabled" = lock false;
            "browser.findBar.suggest.enabled" = lock true;
            "browser.urlbar.showSearchSuggestionsFirst" = lock true;
            "browser.urlbar.speculativeConnect.enabled" = lock false;
            "browser.places.speculativeConnect.enabled" = lock false;
            "browser.aboutConfig.showWarning" = lock false;
            "browser.tabs.warnOnClose" = lock false;
            "browser.tabs.allow_transparent_browser" = lock false;
            "browser.tabs.hoverPreview.enabled" = lock true;
            "browser.safebrowsing.downloads.remote.enabled" = lock false;

            # DOM & IPC settings
            "dom.storage.default_quota" = lock 20480;
            "dom.storage.shadow_writes" = lock true;
            "dom.ipc.processCount" = lock 8;
            "dom.ipc.keepProcessesAlive.web" = lock 4;
            "dom.element.animate.enabled" = lock true;
            "dom.battery.enabled" = lock false;
            "dom.webgpu.enabled" = lock true;

            # GFX & Rendering
            "gfx.content.skia-font-cache-size" = lock 32;
            "gfx.webrender.all" = lock true;
            "gfx.webrender.enabled" = lock true;
            "gfx.webrender.compositor" = lock true;
            "gfx.webrender.precache-shaders" = lock true;
            "gfx.webrender.software" = lock false;
            "gfx.webrender.layer-compositor" = lock true;
            "gfx.canvas.accelerated.cache-items" = lock 32768;
            "gfx.canvas.accelerated.cache-size" = lock 4096;
            "gfx.canvas.max-size" = lock 16384;
            "layers.acceleration.force-enabled" = lock true;
            "layout.frame_rate" = lock (-1);
            "webgl.max-size" = lock 16384;
            "webgl.force-enabled" = lock true;

            # UI & Accessibility
            "ui.submenuDelay" = lock 0;
            "accessibility.force_disabled" = lock 1;
            "general.smoothScroll" = lock true;
            "general.autoScroll" = lock true;

            # Media settings
            "image.mem.max_decoded_image_kb" = lock 512000;
            "image.cache.size" = lock 10485760;
            "image.mem.decode_bytes_at_a_time" = lock 65536;
            "image.mem.shared.unmap.min_expiration_ms" = lock 90000;
            "media.memory_cache_max_size" = lock 1048576;
            "media.memory_caches_combined_limit_kb" = lock 4194304;
            "media.cache_readahead_limit" = lock 600;
            "media.cache_resume_threshold" = lock 300;
            "dom.media.webcodecs.h265.enabled" = lock true;
            "media.videocontrols.picture-in-picture.video-toggle.enabled" = lock false;
            "media.videocontrols.picture-in-picture.enable-when-switching-tabs.enabled" = lock false;
            "media.ffmpeg.vaapi.enabled" = lock true;
            "media.hardware-video-decoding.force-enabled" = lock true;
            "media.wmf.zero-copy-nv12-textures-force-enabled" = lock true;

            # Linux & Wayland
            "widget.wayland.opaque-region.enabled" = lock true;
            "widget.wayland.fractional-scale.enabled" = lock true;
            "widget.gtk.rounded-bottom-corners.enabled" = lock false;
            "zen.widget.linux.transparency" = lock true;

            # System
            "toolkit.legacyUserProfileCustomizations.stylesheets" = lock true;
          };
        };
      };

      xdg.configFile."zen/${config.home.username}/chrome" = catppuccin;
    };
}
