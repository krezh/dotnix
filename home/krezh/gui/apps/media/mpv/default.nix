{ pkgs, ... }:
{
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
}
