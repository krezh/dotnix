{
  flake.modules.homeManager.media = {pkgs, ...}: {
    home.packages = with pkgs; [
      spotify
      vlc
      celluloid
      plex-desktop
      jellyfin-media-player
      tsukimi
      ueberzugpp
      jellyflix
      vipsdisp
      showtime
      fladder
    ];

    programs.ncspot = {
      enable = true;
      package = pkgs.ncspot.override {
        withCover = true;
        withMPRIS = true;
        withALSA = true;
        withNotify = true;
      };
      settings = {
        bitrate = 320;
        cover_max_scale = 2.0;
        flip_status_indicators = true;
        use_nerdfont = true;
        notify = true;
        theme = {
          background = "#1e1e2e";
          primary = "#cdd6f4";
          secondary = "#6c7086";
          title = "#a6e3a1";
          playing = "#a6e3a1";
          playing_selected = "#94e2d5";
          playing_bg = "#1e1e2e";
          highlight = "#b4befe";
          highlight_bg = "#313244";
          error = "#eba0ac";
          error_bg = "#1e1e2e";
          statusbar = "#a6e3a1";
          statusbar_progress = "#b4befe";
          statusbar_bg = "#313244";
          cmdline = "#cdd6f4";
          cmdline_bg = "#1e1e2e";
          search_match = "#f9e2af";
        };
      };
    };

    programs.mpv = {
      enable = true;
      scripts = with pkgs.mpvScripts; [
        modernx
        inhibit-gnome
        sponsorblock
        thumbfast
        mpv-cheatsheet
        quality-menu
      ];

      scriptOpts = {
        modernx = {
          scalewindowed = 1;
          scalefullscreen = 1;
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
        sub-font = "inter";
        sub-font-size = 20;
        sub-border-size = 1.5;
        sub-pos = 95;
        sub-auto = "fuzzy";
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
}
