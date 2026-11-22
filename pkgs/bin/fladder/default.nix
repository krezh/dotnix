{
  lib,
  pkgs,
}:
let
  pname = "fladder";
  # renovate: datasource=github-releases depName=DonutWare/Fladder
  version = "0.12.1";
  src = pkgs.fetchurl {
    url = "https://github.com/DonutWare/Fladder/releases/download/v${version}/Fladder-Linux-${version}.AppImage";
    hash = "sha256-9qQOonbhw+hDy0nqsfhA1yU02fsheFnhcUi5WxD3sTQ=";
  };

  appimageContents = pkgs.appimageTools.extract { inherit pname version src; };
in
pkgs.appimageTools.wrapType2 {
  inherit pname version src;

  extraPkgs = pkgs: [
    pkgs.mpv
    pkgs.libepoxy
    pkgs.lz4
  ];

  extraInstallCommands = ''
    install -Dm444 ${appimageContents}/nl.jknaapen.fladder.desktop $out/share/applications/${pname}.desktop
    substituteInPlace $out/share/applications/${pname}.desktop \
      --replace-fail 'Exec=fladder' 'Exec=${pname}'
    install -Dm444 ${appimageContents}/fladder_icon_desktop.png $out/share/icons/hicolor/256x256/apps/fladder_icon_desktop.png
  '';

  meta = with lib; {
    description = "A cross-platform Jellyfin Frontend built on Flutter";
    homepage = "https://github.com/DonutWare/Fladder";
    changelog = "https://github.com/DonutWare/Fladder/releases/tag/v${version}";
    license = licenses.gpl3Only;
    platforms = [ "x86_64-linux" ];
    mainProgram = pname;
  };
}
