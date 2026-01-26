{
  flake.modules.homeManager.xdg-settings = {
    pkgs,
    lib,
    ...
  }: let
    defaultBrowser = "zen-beta.desktop";
    defaultImageViewer = "org.libvips.vipsdisp.desktop";
    defaultVideoPlayer = "org.gnome.Showtime.desktop";
    defaultTextEditor = "dev.zed.Zed.desktop";
    defaultFileManager = "org.gnome.Nautilus.desktop";
    defaultArchiveManager = "org.gnome.FileRoller.desktop";
    defaultMailClient = "org.gnome.Geary.desktop";

    mimeList = builtins.readFile "${pkgs.shared-mime-info}/share/mime/types";
    allMimes = lib.splitString "\n" mimeList;

    defaultsFor = prefix: app: lib.genAttrs (builtins.filter (m: lib.strings.hasPrefix prefix m) allMimes) (_: app);

    mediaDefaults = lib.mkMerge [
      (defaultsFor "image/" defaultImageViewer)
      (defaultsFor "video/" defaultVideoPlayer)
      (defaultsFor "audio/" defaultVideoPlayer)
    ];

    manualDefaults = {
      "application/pdf" = "org.gnome.Evince.desktop";
      "x-scheme-handler/mailto" = defaultMailClient;
      "text/html" = defaultBrowser;
      "x-scheme-handler/http" = defaultBrowser;
      "x-scheme-handler/https" = defaultBrowser;
      "x-scheme-handler/chrome" = defaultBrowser;
      "x-scheme-handler/about" = defaultBrowser;
      "x-scheme-handler/ftp" = defaultBrowser;
      "x-scheme-handler/unknown" = defaultBrowser;
      "x-scheme-handler/webcal" = defaultBrowser;
      "application/x-extension-htm" = defaultBrowser;
      "application/x-extension-html" = defaultBrowser;
      "application/x-extension-shtml" = defaultBrowser;
      "application/x-extension-xht" = defaultBrowser;
      "application/x-extension-xhtml" = defaultBrowser;
      "application/xhtml+xml" = defaultBrowser;
      "text/plain" = defaultTextEditor;
      "text/markdown" = defaultTextEditor;
      "application/json" = defaultTextEditor;
      "application/xml" = defaultTextEditor;
      "application/x-yaml" = defaultTextEditor;
      "inode/directory" = defaultFileManager;
      "application/zip" = defaultArchiveManager;
      "application/x-tar" = defaultArchiveManager;
      "application/x-7z-compressed" = defaultArchiveManager;
      "application/x-rar" = defaultArchiveManager;
      "application/gzip" = defaultArchiveManager;
      "application/x-bzip2" = defaultArchiveManager;
      "application/x-xz" = defaultArchiveManager;
      "application/x-lzip" = defaultArchiveManager;
      "application/x-lzma" = defaultArchiveManager;
      "application/x-zstd" = defaultArchiveManager;
      "application/x-compress" = defaultArchiveManager;
      "application/x-bzip" = defaultArchiveManager;
      "application/x-lzop" = defaultArchiveManager;
      "application/x-lz4" = defaultArchiveManager;
    };
  in {
    home.sessionVariables = {
      DEFAULT_BROWSER = "${pkgs.xdg-utils}/bin/xdg-open";
    };

    xdg.enable = true;

    xdg.mimeApps = {
      enable = true;
      defaultApplications = lib.mkMerge [
        mediaDefaults
        manualDefaults
      ];
    };
  };
}
