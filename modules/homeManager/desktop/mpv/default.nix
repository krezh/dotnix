{
  config,
  osConfig,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.hmModules.desktop.mpv;
  inherit (lib) mkEnableOption mkOption mkIf;

  uuidFile = "${config.xdg.stateHome}/jellyfin-mpv-shim/client-uuid";
  clientUUID = if builtins.pathExists uuidFile then builtins.readFile uuidFile else null;
  defaultPlayerName =
    if cfg.jellyfinMpvShim.playerName != null then
      cfg.jellyfinMpvShim.playerName
    else
      osConfig.networking.hostName;
in
{
  options.hmModules.desktop.mpv = {
    enable = mkEnableOption "Enable MPV media player";

    # Base MPV configuration
    mpv = {
      scripts = mkOption {
        type = lib.types.listOf lib.types.package;
        default = with pkgs.mpvScripts; [
          modernx
          inhibit-gnome
          sponsorblock
          thumbfast
          mpv-cheatsheet
          quality-menu
        ];
        description = "MPV scripts to enable.";
      };

      modernxOptions = mkOption {
        type = lib.types.attrs;
        default = {
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
          bottomhover = true;
          showontop = true;
          raisesubswithosc = false;
        };
        description = "Options for ModernX OSC.";
      };

      bindings = mkOption {
        type = lib.types.attrs;
        default = {
          WHEEL_UP = "add volume 5";
          WHEEL_DOWN = "add volume -5";
          "Ctrl+WHEEL_UP" = "add speed 0.1";
          "Ctrl+WHEEL_DOWN" = "add speed -0.1";
          "MBTN_MID" = "cycle mute";
          F1 = "af toggle acompressor=ratio=4; af toggle loudnorm";
          E = "add panscan -0.1";
          l = "no-osd seek 100 absolute-percent";
        };
        description = "Key bindings for MPV.";
      };
    };

    # Jellyfin MPV Shim configuration (clearly separated)
    jellyfinMpvShim = {
      enable = mkEnableOption "Enable Jellyfin MPV Shim integration.";

      package = mkOption {
        type = pkgs.lib.types.package;
        default = pkgs.jellyfin-mpv-shim;
        description = "Package to use for Jellyfin MPV Shim.";
      };

      playerName = mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Player name to use in Jellyfin MPV Shim. Defaults to hostname if unset.";
      };
    };
  };

  config = mkIf cfg.enable {
    # MPV base
    programs.mpv = {
      enable = true;
      scripts = cfg.mpv.scripts;
      scriptOpts = {
        modernx = cfg.mpv.modernxOptions;
      };
      bindings = cfg.mpv.bindings;
      config = {
        profile = "high-quality";
        hwdec = "auto-safe";
        vo = "gpu-next";
        video-sync = "display-resample";
        interpolation = true;
        tscale = "oversample";
        ytdl-format = "bestvideo+bestaudio";

        save-position-on-quit = false;
        osc = "no"; # ModernX provides OSC

        sub-font = "inter";
        sub-font-size = 20;
        sub-border-size = 1.5;
        sub-pos = 95;
        sub-auto = "fuzzy";
      };
    };

    # Jellyfin MPV Shim
    home.packages = lib.mkIf cfg.jellyfinMpvShim.enable [ cfg.jellyfinMpvShim.package ];

    # Persistent client UUID
    home.activation.initJellyfinShimUUID = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
      if [ ! -f "${uuidFile}" ]; then
        mkdir -p "$(dirname "${uuidFile}")"
        ${pkgs.util-linux}/bin/uuidgen > "${uuidFile}"
      fi
    '';

    xdg.configFile = lib.mkIf cfg.jellyfinMpvShim.enable {
      "jellyfin-mpv-shim/mpv.conf".source = config.xdg.configFile."mpv/mpv.conf".source;

      "jellyfin-mpv-shim/input.conf".source = config.xdg.configFile."mpv/input.conf".source;

      "jellyfin-mpv-shim/conf.json".text = builtins.toJSON {
        allow_transcode_to_h265 = false;
        always_transcode = false;
        audio_output = "hdmi";
        auto_play = true;
        check_updates = false;
        client_uuid = clientUUID;
        connect_retry_mins = 0;
        direct_paths = false;
        discord_presence = false;
        display_mirroring = false;
        enable_gui = true;
        enable_osc = true;
        force_audio_codec = null;
        force_set_played = false;
        force_video_codec = null;
        fullscreen = true;
        health_check_interval = 300;
        idle_cmd = null;
        idle_cmd_delay = 60;
        idle_ended_cmd = null;
        idle_when_paused = false;
        ignore_ssl_cert = false;
        kb_debug = "~";
        kb_fullscreen = "f";
        kb_kill_shader = "k";
        kb_menu = "c";
        kb_menu_down = "down";
        kb_menu_esc = "esc";
        kb_menu_left = "left";
        kb_menu_ok = "enter";
        kb_menu_right = "right";
        kb_menu_up = "up";
        kb_next = ">";
        kb_pause = "space";
        kb_prev = "<";
        kb_stop = "q";
        kb_unwatched = "u";
        kb_watched = "w";
        lang = null;
        lang_filter = "und,eng,jpn,mis,mul,zxx";
        lang_filter_audio = false;
        lang_filter_sub = false;
        local_kbps = 2147483;
        log_decisions = false;
        media_ended_cmd = null;
        media_key_seek = false;
        media_keys = true;
        menu_mouse = true;
        mpv_ext = true;
        mpv_ext_path = "mpv";
        mpv_ext_ipc = null;
        mpv_ext_no_ovr = false;
        mpv_ext_start = true;
        mpv_log_level = "info";
        notify_updates = false;
        play_cmd = null;
        playback_timeout = 30;
        player_name = defaultPlayerName;
        pre_media_cmd = null;
        prefer_transcode_to_h265 = false;
        raise_mpv = true;
        remote_direct_paths = false;
        remote_kbps = 10000;
        sanitize_output = true;
        screenshot_dir = null;
        screenshot_menu = true;
        seek_down = -60;
        seek_h_exact = false;
        seek_left = -5;
        seek_right = 5;
        seek_up = 60;
        seek_v_exact = false;
        shader_pack_custom = false;
        shader_pack_enable = true;
        shader_pack_profile = null;
        shader_pack_remember = true;
        shader_pack_subtype = "lq";
        skip_credits_always = false;
        skip_credits_enable = true;
        skip_intro_always = false;
        skip_intro_enable = true;
        stop_cmd = null;
        stop_idle = false;
        subtitle_color = "#FFFFFFFF";
        subtitle_position = "bottom";
        subtitle_size = 100;
        svp_enable = false;
        svp_socket = null;
        svp_url = "http://127.0.0.1:9901/";
        sync_attempts = 5;
        sync_max_delay_skip = 300;
        sync_max_delay_speed = 50;
        sync_method_thresh = 2000;
        sync_osd_message = true;
        sync_revert_seek = true;
        sync_speed_attempts = 3;
        sync_speed_time = 1000;
        thumbnail_enable = true;
        thumbnail_osc_builtin = false;
        thumbnail_preferred_size = 320;
        tls_client_cert = null;
        tls_client_key = null;
        tls_server_ca = null;
        transcode_4k = false;
        transcode_av1 = false;
        transcode_dolby_vision = true;
        transcode_hdr = false;
        transcode_hevc = false;
        transcode_hi10p = false;
        transcode_warning = true;
        use_web_seek = false;
        write_logs = false;
      };
    };
  };
}
