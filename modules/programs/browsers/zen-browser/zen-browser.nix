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

                # Fix dark-on-dark select dropdown: set panel item hover/active colors explicitly
                cat >> $out/userChrome.css << 'EOF'

        @media (prefers-color-scheme: dark) {
          :root {
            --panel-color: #cdd6f4 !important;
            --panel-background: #1e1e2e !important;
          }
          #ContentSelectDropdown > menupopup {
            color: #cdd6f4 !important;
            color-scheme: dark !important;
          }
        }
        EOF
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

            # "javascript.options.mem.high_water_mark" = 128;

            # "content.maxtextrun" = 8191;
            # "content.interrupt.parsing" = true;
            # "content.notify.ontimer" = true;
            # "content.notify.interval" = 50000;
            # "content.max.tokenizing.time" = 2000000;
            # "content.switch.threshold" = 300000;

            # Layout & rendering
            # "nglayout.initialpaint.delay" = 5;

            "general.smoothScroll.msdPhysics.enabled" = false;
            "general.smoothScroll.currentVelocityWeighting" = 0;
            "general.smoothScroll.stopDecelerationWeighting" = 1;
            "general.smoothScroll.mouseWheel.durationMaxMS" = 150;
            "general.smoothScroll.mouseWheel.durationMinMS" = 50;
            "apz.overscroll.enabled" = false;

            # "fission.autostart" = true;

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
            "zen.window-sync.enabled" = true;
            "zen.window-sync.sync-only-pinned-tabs" = true;
            "zen.view.use-native-titlebar" = true;
            "reader.parse-on-load.enabled" = false;
            "zen.tabs.dnd-open-blank-window" = false;
            "zen.widget.linux.transparency" = true;
          };
        };
        policies = {
          AutofillAddressEnabled = false;
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
            "browser.uidensity" = lock 0;
            "browser.ml.chat.enabled" = lock false;
            "browser.urlbar.speculativeConnect.enabled" = lock false;
            "browser.places.speculativeConnect.enabled" = lock false;
            "browser.aboutConfig.showWarning" = lock false;
            "browser.tabs.min_inactive_duration_before_unload" = lock 600000;
            "browser.tabs.warnOnClose" = lock false;
            "browser.tabs.hoverPreview.enabled" = lock true;
            "browser.safebrowsing.downloads.remote.enabled" = lock false;
            "dom.battery.enabled" = lock false;

            # UI & Accessibility
            "ui.submenuDelay" = lock 0;
            "accessibility.force_disabled" = lock 1;
            "general.smoothScroll" = lock true;
            "general.autoScroll" = lock true;

            # Media settings
            "media.videocontrols.picture-in-picture.video-toggle.enabled" = lock false;
            "media.videocontrols.picture-in-picture.enable-when-switching-tabs.enabled" = lock false;

            # Linux & Wayland
            "widget.wayland.opaque-region.enabled" = lock true;
            "widget.wayland.fractional-scale.enabled" = lock true;
            "widget.gtk.rounded-bottom-corners.enabled" = lock true;

            # Transparancy
            "browser.tabs.allow_transparent_browser" = lock false;
            "widget.transparent-windows" = lock true;

            # System
            "toolkit.legacyUserProfileCustomizations.stylesheets" = lock true;
          };
        };
      };

      xdg.configFile."zen/${config.home.username}/chrome" = catppuccin;
    };
}
