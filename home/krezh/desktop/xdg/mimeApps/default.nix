{ pkgs, lib, ... }:
let
  defaultBrowser = "zen-beta.desktop";
  defaultImageViewer = "org.libvips.vipsdisp.desktop";
  defaultVideoPlayer = "mpv.desktop";
  defaultTextEditor = "dev.zed.Zed.desktop";
  defaultFileManager = "org.gnome.Nautilus.desktop";
  defaultArchiveManager = "org.gnome.FileRoller.desktop";

  # Path to the freedesktop.org MIME type list
  mimeList = builtins.readFile "${pkgs.shared-mime-info}/share/mime/types";
  allMimes = lib.splitString "\n" mimeList;

  # Helper: build defaults for all MIME types with given prefix
  defaultsFor =
    prefix: app: lib.genAttrs (builtins.filter (m: lib.strings.hasPrefix prefix m) allMimes) (_: app);

  mediaDefaults = lib.mkMerge [
    (defaultsFor "image/" defaultImageViewer)
    (defaultsFor "video/" defaultVideoPlayer)
    (defaultsFor "audio/" defaultVideoPlayer)
  ];

  manualDefaults = {
    "application/pdf" = "org.gnome.Evince.desktop";
    # Browser
    "text/html" = defaultBrowser;
    "x-scheme-handler/http" = defaultBrowser;
    "x-scheme-handler/https" = defaultBrowser;
    "x-scheme-handler/ftp" = defaultBrowser;
    "x-scheme-handler/mailto" = defaultBrowser;
    "x-scheme-handler/webcal" = defaultBrowser;
    "x-scheme-handler/about" = defaultBrowser;
    "x-scheme-handler/unknown" = defaultBrowser;
    # Editor
    "text/plain" = defaultTextEditor;
    "text/markdown" = defaultTextEditor;
    "application/json" = defaultTextEditor;
    "application/xml" = defaultTextEditor;
    "application/x-yaml" = defaultTextEditor;
    # File manager
    "inode/directory" = defaultFileManager;
    # Archive
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

in
{
  home = {
    sessionVariables = {
      DEFAULT_BROWSER = "${pkgs.xdg-utils}/bin/xdg-open";
    };
  };
  xdg = {
    mimeApps = {
      enable = true;
      defaultApplications = lib.mkMerge [
        mediaDefaults
        manualDefaults
      ];
    };
  };
}
