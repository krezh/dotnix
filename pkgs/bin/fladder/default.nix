{
  lib,
  appimageTools,
  fetchurl,
}:

let
  pname = "fladder";
  # renovate: datasource=github-releases depName=DonutWare/Fladder
  version = "0.9.0";

  src = fetchurl {
    url = "https://github.com/DonutWare/Fladder/releases/download/v${version}/Fladder-Linux-${version}.AppImage";
    hash = "sha256-L9dyqEGrMlGW6C7Jj4nhM5X/DlJ3vDNL4pSlsVel8Iw=";
  };
in
appimageTools.wrapType2 {
  inherit pname version src;

  extraPkgs = pkgs: [
    pkgs.mpv
    pkgs.libepoxy
  ];

  meta = with lib; {
    description = "A cross-platform Jellyfin Frontend built on Flutter";
    homepage = "https://github.com/DonutWare/Fladder";
    license = licenses.gpl3Only;
    platforms = [ "x86_64-linux" ];
    mainProgram = pname;
  };
}
