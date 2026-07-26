{ inputs, ... }:
{
  flake.modules.homeManager.media =
    { pkgs, ... }:
    let
      spicePkgs = inputs.spicetify-nix.legacyPackages.${pkgs.system};
    in
    {
      imports = [ inputs.spicetify-nix.homeManagerModules.spicetify ];

      home.packages = with pkgs; [
        plex-desktop
        jellyfin-media-player
        jellyfin-mpv-shim
        tsukimi
        ueberzugpp
        vipsdisp
        fladder
      ];

      programs = {
        spicetify = {
          enable = true;
          enabledExtensions = with spicePkgs.extensions; [
            adblockify
            hidePodcasts
            shuffle
          ];
          theme = spicePkgs.themes.catppuccin;
          colorScheme = "mocha";
        };

        mpv = {
          enable = true;
          scripts = with pkgs.mpvScripts; [
            # modernx
            uosc
            inhibit-gnome
            sponsorblock
            thumbfast
            quality-menu
            mpris
          ];

          scriptOpts = {
            modernx = {
              scalewindowed = 0.5;
              scalefullscreen = 0.5;
              fadeduration = 150;
              hidetimeout = 5000;
              donttimeoutonpause = true;
              OSCfadealpha = 75;
              showtitle = true;
              showinfo = true;
              windowcontrols = false;
              volumecontrol = true;
              compactmode = false;
              raisesubswithosc = false;
            };
            uosc = {
              timeline_size = 25;
              timeline_persistency = "paused,audio";
              progress = "always";
              progress_size = 4;
              progress_line_width = 4;
              controls = "subtitles,<has_many_audio>audio,<has_many_video>video,<has_many_edition>editions,<stream>stream-quality";
              top_bar = "never";
              refine = "text_width";
            };
            thumbfast = {
              spawn_first = true;
              network = true;
              hwdec = true;
            };
          };

          config = {
            profile = "high-quality";
            hwdec = "auto-safe";
            vo = "gpu-next";
            video-sync = "display-resample";
            interpolation = true;
            tscale = "oversample";
            ytdl-format = "bestvideo+bestaudio";
            save-position-on-quit = false;
            osc = "no";
            sub-font = "Rubik";
            sub-font-size = 20;
            sub-border-size = 1.5;
            sub-pos = 95;
            sub-auto = "fuzzy";
            keep-open = true;
          };

          bindings = {
            WHEEL_UP = "add volume 5";
            WHEEL_DOWN = "add volume -5";
            "Ctrl+WHEEL_UP" = "add speed 0.1";
            "Ctrl+WHEEL_DOWN" = "add speed -0.1";
            "MBTN_MID" = "cycle mute";
            F1 = "af toggle acompressor=ratio=4; af toggle loudnorm";
            E = "add panscan -0.1";
            l = "no-osd seek 100 absolute-percent";
          };
        };
      };
    };
}
