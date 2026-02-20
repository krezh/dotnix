{
  lib,
  appimageTools,
  fetchurl,
}:
let
  pname = "fladder";
  # renovate: datasource=github-releases depName=DonutWare/Fladder
  version = "0.10.0";
  src = fetchurl {
    url = "https://github.com/DonutWare/Fladder/releases/download/v${version}/Fladder-Linux-${version}.AppImage";
    hash = "sha256-LU2IFM7Jt/Cbr3BNMgUrNFER0RD/oTlBoG1a+znsC1M=";
  };

  appimageContents = appimageTools.extractType2 { inherit pname version src; };
in
appimageTools.wrapType2 {
  inherit pname version src;

  extraPkgs = pkgs: [
    pkgs.mpv
    pkgs.libepoxy
  ];

  extraInstallCommands = ''
    install -Dm444 ${appimageContents}/nl.jknaapen.fladder.desktop $out/share/applications/${pname}.desktop
    substituteInPlace $out/share/applications/${pname}.desktop \
      --replace-fail 'Exec=Fladder' 'Exec=${pname}'
    install -Dm444 ${appimageContents}/fladder_icon_desktop.png $out/share/icons/hicolor/256x256/apps/fladder_icon_desktop.png
  '';

  meta = with lib; {
    description = "A cross-platform Jellyfin Frontend built on Flutter";
    homepage = "https://github.com/DonutWare/Fladder";
    license = licenses.gpl3Only;
    platforms = [ "x86_64-linux" ];
    mainProgram = pname;
  };
}
