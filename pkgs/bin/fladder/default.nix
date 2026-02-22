{
  lib,
  appimageTools,
  fetchurl,
}:
let
  pname = "fladder";
  # renovate: datasource=github-releases depName=DonutWare/Fladder
  version = "v0.10.1";
  src = fetchurl {
    url = "https://github.com/DonutWare/Fladder/releases/download/${version}/Fladder-Linux-${lib.removePrefix "v" version}.AppImage";
    hash = "sha256-TGniwr+a2iB9a38/F1CXlrhcYt45FaM8qAUrBXGekx8=";
  };

  appimageContents = appimageTools.extractType2 { inherit pname version src; };
in
appimageTools.wrapType2 {
  inherit pname version src;

  extraPkgs = pkgs: [
    pkgs.mpv
    pkgs.libepoxy
    pkgs.lz4
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
