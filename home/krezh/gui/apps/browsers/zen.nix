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
            urls = [
              {
                template = "https://kagi.com/search?q={searchTerms}";
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
        # https://github.com/Eratas/rapidfox/wiki/Rapidfox-Guide-v7.3
        # MANDATORY UNLOCK
        "browser.preferences.defaultPerformanceSettings.enabled" = false; # Unlock manual performance tuning

        # NETWORK - CONNECTION POOL
        "network.http.max-connections" = 1200; # Total simultaneous connections (default 256)
        "network.http.max-persistent-connections-per-server" = 8; # Keep-alive connections per host
        "network.http.max-urgent-start-excessive-connections-per-host" = 5; # Extra connections for high-priority content
        "network.http.request.max-start-delay" = 5; # Max delay before HTTP request dispatch

        # NETWORK - REQUEST PACING
        "network.http.pacing.requests.enabled" = false; # Disable pacing for faster bursts
        "network.http.pacing.requests.burst" = 32; # HTTP requests per burst
        "network.http.pacing.requests.min-parallelism" = 10; # Min active connections before pacing

        # NETWORK - DNS & TLS
        "network.ssl_tokens_cache_capacity" = 32768; # TLS session ticket cache for faster HTTPS
        "network.http.http3.enabled" = true; # Enable HTTP/3

        # NETWORK - PREFETCH & SPECULATION (disabled for efficiency)
        "network.http.speculative-parallel-limit" = 0; # Disable speculative connections
        "network.dns.disablePrefetch" = true; # Stop DNS prefetching
        "network.dns.disablePrefetchFromHTTPS" = true; # Stop DNS prefetch from HTTPS
        "network.prefetch-next" = false; # Disable next-page prefetching
        "network.predictor.enabled" = false; # Disable predictive prefetching
        "network.predictor.enable-prefetch" = false; # Block predictor prefetching
        "browser.urlbar.speculativeConnect.enabled" = false; # Stop address bar speculation
        "browser.places.speculativeConnect.enabled" = false; # Stop history/bookmark speculation

        # MEMORY - JAVASCRIPT
        "javascript.options.mem.high_water_mark" = 128; # GC pressure threshold (128MB)

        # MEMORY - BROWSER CACHE
        "browser.cache.disk.enable" = false; # Disable disk cache, RAM only
        "browser.cache.disk.capacity" = 0; # Disk cache size 0 when disabled
        "browser.cache.memory.capacity" = 131072; # Memory cache 128MB
        "browser.cache.disk.smart_size.enabled" = false; # Disable dynamic cache sizing
        "browser.cache.memory.max_entry_size" = 32768; # Max single cache entry 32MB
        "browser.cache.disk.metadata_memory_limit" = 16384; # Metadata memory pool 16MB
        "browser.cache.max_shutdown_io_lag" = 100; # Max shutdown I/O wait 100ms

        # MEMORY - IMAGE PROCESSING
        "image.mem.max_decoded_image_kb" = 512000; # Max decoded image memory 500MB
        "image.cache.size" = 10485760; # Image cache 10MB
        "image.mem.decode_bytes_at_a_time" = 65536; # Decode chunk size 64KB
        "image.mem.shared.unmap.min_expiration_ms" = 90000; # Image memory unmap time 1.5min

        # MEMORY - MEDIA CACHE
        "media.memory_cache_max_size" = 1048576; # Media RAM cache 1GB
        "media.memory_caches_combined_limit_kb" = 4194304; # Total media cache 4GB
        "media.cache_readahead_limit" = 600; # Pre-buffer 10 minutes
        "media.cache_resume_threshold" = 300; # Resume threshold 5 minutes

        # SESSION & STORAGE
        "dom.storage.default_quota" = 20480; # Local storage quota 20MB per origin
        "dom.storage.shadow_writes" = true; # Enable shadow writes for LSNG
        "browser.sessionstore.interval" = 60000; # Session save interval 1 minute
        "browser.sessionhistory.max_total_viewers" = 10; # Cached pages for back/forward
        "browser.sessionstore.max_tabs_undo" = 10; # Recently closed tabs limit
        "browser.sessionstore.max_entries" = 10; # Per-tab history limit
        "browser.tabs.min_inactive_duration_before_unload" = 600000; # Tab auto unload delay

        # CONTENT PROCESSING
        "content.maxtextrun" = 8191; # Max text buffer parse size
        "content.interrupt.parsing" = true; # Allow parsing interruption for UI
        "content.notify.ontimer" = true; # Timer-based content notifications
        "content.notify.interval" = 50000; # Content notification delay 0.05s
        "content.max.tokenizing.time" = 2000000; # Max parsing time 2s
        "content.switch.threshold" = 300000; # UI responsiveness threshold 0.3s

        # LAYOUT & RENDERING
        "layout.frame_rate" = -1; # Auto frame rate detection
        "nglayout.initialpaint.delay" = 5; # Initial paint delay 5ms

        # FONT RENDERING
        "gfx.content.skia-font-cache-size" = 32; # Font cache 32MB

        # GPU - WEBRENDER
        "gfx.webrender.all" = true; # WebRender for all content
        "gfx.webrender.enabled" = true; # Enable WebRender engine
        "gfx.webrender.compositor" = true; # GPU-based frame composition
        "gfx.webrender.precache-shaders" = true; # Precompile GPU shaders
        "gfx.webrender.software" = false; # Disable CPU fallback
        "gfx.webrender.layer-compositor" = true; # Layer compositor

        # GPU - HARDWARE ACCELERATION
        "layers.acceleration.force-enabled" = true; # Force GPU acceleration

        # GPU - CANVAS & WEBGL
        "gfx.canvas.accelerated.cache-items" = 32768; # GPU-cached canvas items
        "gfx.canvas.accelerated.cache-size" = 4096; # Canvas cache 4MB GPU
        "gfx.canvas.max-size" = 16384; # Max canvas dimension 16384px
        "webgl.max-size" = 16384; # Max WebGL texture size
        "webgl.force-enabled" = true; # Force WebGL
        "dom.webgpu.enabled" = true; # Enable WebGPU API

        # UI - INTERFACE
        "ui.submenuDelay" = 0; # Instant sub-menu opening
        "browser.uidensity" = 0; # UI density (0=normal, 1=compact, 2=large)
        "dom.element.animate.enabled" = true; # Web Animations API

        # UI - SCROLLING (optimized)
        "general.smoothScroll" = true; # Enable smooth scrolling
        "general.smoothScroll.msdPhysics.enabled" = false; # Disable problematic physics
        "general.smoothScroll.currentVelocityWeighting" = 0; # No velocity smoothing
        "general.smoothScroll.stopDecelerationWeighting" = 1; # Immediate scroll stop
        "general.smoothScroll.mouseWheel.durationMaxMS" = 150; # Max scroll animation 150ms
        "general.smoothScroll.mouseWheel.durationMinMS" = 50; # Min scroll animation 50ms
        "apz.overscroll.enabled" = false; # Disable bouncy overscroll
        # "mousewheel.min_line_scroll_amount" = 18; # Scroll distance per tick
        # "mousewheel.scroll_series_timeout" = 10; # Scroll event grouping 10ms
        "general.autoScroll" = true; # Auto scroll

        # PROCESS MANAGEMENT
        "dom.ipc.processCount" = 8; # Content processes
        "dom.ipc.keepProcessesAlive.web" = 4; # Keep processes alive for fast tabs
        "accessibility.force_disabled" = 1; # Disable accessibility (saves resources)

        # SITE ISOLATION
        "fission.autostart" = true; # Site isolation for security

        # MEDIA - CODECS
        "dom.media.webcodecs.h265.enabled" = true; # H.265/HEVC in WebCodecs
        "media.videocontrols.picture-in-picture.video-toggle.enabled" = false; # PiP toggle
        "media.videocontrols.picture-in-picture.enable-when-switching-tabs.enabled" = false; # Auto-PiP on tab switch

        # MEDIA - LINUX HARDWARE ACCELERATION
        "media.ffmpeg.vaapi.enabled" = true; # VAAPI hardware decoding (Linux)
        "media.hardware-video-decoding.force-enabled" = true; # Force HW decoding (Linux, moderate risk)
        "media.wmf.zero-copy-nv12-textures-force-enabled" = true; # Zero-copy textures

        # LINUX - WAYLAND
        "widget.wayland.opaque-region.enabled" = true; # Optimize Wayland rendering
        "widget.wayland.fractional-scale.enabled" = true; # Fractional scaling support
        "widget.gtk.rounded-bottom-corners.enabled" = false; # GTK corners fix

        # PRIVACY - TRACKING PROTECTION
        "privacy.trackingprotection.enabled" = false; # Disable built-in TP (using policy instead, moderate risk)
        "privacy.query_stripping.enabled" = true; # Strip tracking params from URLs
        "privacy.query_stripping.enabled.pbmode" = true; # Strip params in private mode
        "privacy.spoof_english" = 1; # Spoof English
        "privacy.firstparty.isolate" = true; # First-party isolation
        "network.cookie.cookieBehavior" = 5; # Cookie behavior
        "dom.battery.enabled" = false; # Disable battery API

        # PRIVACY - REFERRER
        "network.http.referer.XOriginPolicy" = 0; # Full referrer for compatibility
        "network.http.referer.XOriginTrimmingPolicy" = 0; # Full referrer info

        # PRIVACY - NETWORK STATE
        "privacy.partition.network_state" = false; # Share network cache globally

        # SECURITY - SAFE BROWSING
        "browser.safebrowsing.downloads.remote.enabled" = false; # Disable Google remote scan

        # AI & SUGGESTIONS (disabled)
        "browser.ml.chat.enabled" = false; # Disable AI chat
        "browser.search.suggest.enabled" = false; # Disable search suggestions
        "browser.urlbar.suggest.searches" = false; # Disable URL bar suggestions
        "browser.findBar.suggest.enabled" = false; # Disable find bar suggestions

        # ZEN - THEME & APPEARANCE
        "zen.theme.accent-color" = "#ffffff90"; # Accent color with transparency
        "zen.theme.border-radius" = 0; # Sharp corners
        "zen.theme.content-element-separation" = 0; # No separation
        "zen.theme.gradient" = true; # Gradient theme
        "zen.theme.gradient.show-custom-colors" = true; # Custom gradient colors
        "zen.theme.acrylic-elements" = false; # Acrylic blur effects

        # ZEN - NAVIGATION & URL BAR
        "zen.urlbar.replace-newtab" = true; # Floating URL bar
        "zen.urlbar.behavior" = "float"; # URL bar behavior

        # ZEN - WORKSPACES & TABS
        "zen.workspaces.open-new-tab-if-last-unpinned-tab-is-closed" = true; # Auto-create tab
        "zen.workspaces.continue-where-left-off" = true; # Continue where left off
        "zen.workspaces.show-workspace-indicator" = false; # Hide indicator
        "zen.splitView.enable-tab-drop" = false; # Split view tab drop
        "zen.tabs.show-newtab-vertical" = false; # Vertical new tab

        # ZEN - VIEW & COMPACT MODE
        "zen.view.experimental-rounded-view" = false; # Fix font rendering
        "zen.view.gray-out-inactive-windows" = false; # No gray out
        "zen.view.compact.enable-at-startup" = false; # Compact at startup
        "zen.view.compact.hide-toolbar" = true; # Hide toolbar
        "zen.view.compact.toolbar-flash-popup" = true; # Toolbar flash
        "zen.view.show-newtab-button-top" = false; # New tab button
        "zen.view.window.scheme" = 2; # Auto theme

        # ZEN - MISC
        "zen.watermark.enabled" = false; # Disable watermark
        "zen.mediacontrols.enabled" = false; # Disable media controls
        "zen.welcome-screen.seen" = true; # Welcome screen
        "reader.parse-on-load.enabled" = false; # Disable auto Reader Mode parsing

        # BROWSER - MISC
        "browser.aboutConfig.showWarning" = false; # No about:config warning
        "browser.tabs.warnOnClose" = false; # No close warning
        "browser.tabs.hoverPreview.enabled" = true; # Tab hover preview
        "browser.newtabpage.activity-stream.feeds.topsites" = false; # No top sites
        "browser.topsites.contile.enabled" = false; # No contile
        "browser.formfill.enable" = false; # No form fill
        "browser.download.useDownloadDir" = true; # Use download dir
        "toolkit.legacyUserProfileCustomizations.stylesheets" = true; # Enable userChrome.css
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
