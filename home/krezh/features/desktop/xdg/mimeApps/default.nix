{ pkgs, lib, ... }:
let
  defaultBrowser = "zen-beta.desktop";
  defaultImageViewer = "org.libvips.vipsdisp.desktop";
  defaultVideoPlayer = "io.github.celluloid_player.Celluloid.desktop";

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
    "application/json" = defaultBrowser;
    "text/html" = defaultBrowser;
    "x-scheme-handler/http" = defaultBrowser;
    "x-scheme-handler/https" = defaultBrowser;
    "x-scheme-handler/ftp" = defaultBrowser;
    "x-scheme-handler/file" = defaultBrowser;
    "x-scheme-handler/mailto" = defaultBrowser;
    "x-scheme-handler/webcal" = defaultBrowser;
    "x-scheme-handler/about" = defaultBrowser;
    "x-scheme-handler/unknown" = defaultBrowser;
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
