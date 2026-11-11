{
  lib,
  pkgs,
}:
let
  pname = "wowup-cf";
  # renovate: datasource=github-releases depName=WowUp/WowUp.CF
  version = "2.25.5";
  src = pkgs.fetchurl {
    url = "https://github.com/WowUp/WowUp.CF/releases/download/v${version}/WowUp-CF-${version}.AppImage";
    hash = "sha256-OHeSxDwTWd3DXmeBAAoG/w/Dj7VXhdwCmn2X9PE6SDk=";
  };

  appimageContents = pkgs.appimageTools.extract { inherit pname version src; };
in
pkgs.appimageTools.wrapType2 {
  inherit pname version src;

  extraInstallCommands = ''
    install -Dm444 ${appimageContents}/${pname}.desktop -t $out/share/applications
    substituteInPlace $out/share/applications/${pname}.desktop \
      --replace-fail 'Exec=AppRun' 'Exec=${pname}'
    echo 'MimeType=x-scheme-handler/curseforge;x-scheme-handler/wowup;' >> $out/share/applications/${pname}.desktop
    cp -r ${appimageContents}/usr/share/icons $out/share
  '';

  meta = {
    description = "World of Warcraft addon updater with CurseForge support";
    homepage = "https://wowup.io/";
    changelog = "https://github.com/WowUp/WowUp.CF/releases/tag/v${version}";
    license = lib.licenses.unfree;
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
    platforms = [ "x86_64-linux" ];
    mainProgram = pname;
  };
}
