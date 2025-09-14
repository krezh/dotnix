{
  lib,
  appimageTools,
  fetchurl,
}:

let
  version = "2.20.0";
  pname = "wowup";
  src = fetchurl {
    url = "https://github.com/WowUp/WowUp/releases/download/v${version}/WowUp-${version}.AppImage";
    hash = "sha256-oDlmL/1N+6Q4zBExLlnILL4LSoz+aF2tSA/x+WpsZ4A=";
  };
in
appimageTools.wrapType2 rec {
  inherit pname version src;

  extraInstallCommands = ''
    substituteInPlace $out/share/applications/${pname}.desktop \
      --replace-fail 'Exec=AppRun' 'Exec=${pname}'
  '';

  meta = {
    description = "";
    homepage = "https://github.com/WowUp/WowUp";
    sourceProvenance = with lib.sourceTypes; [ binaryNativeCode ];
    platforms = [ "x86_64-linux" ];
  };
}
