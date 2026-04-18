{ appimageTools, fetchurl }:
appimageTools.wrapType2 rec {
  pname = "helium";
  # renovate: datasource=github-releases depName=imputnet/helium-linux versioning=loose
  version = "0.11.3.2";

  src = fetchurl {
    url = "https://github.com/imputnet/helium-linux/releases/download/${version}/${pname}-${version}-x86_64.AppImage";
    sha256 = "sha256-5gdyKg12ZV2hpf0RL+eoJnawuW/J8NobiG+zEA0IOHA=";
  };

  extraInstallCommands =
    let
      contents = appimageTools.extract { inherit pname version src; };
    in
    ''
      install -m 444 -D ${contents}/${pname}.desktop -t $out/share/applications
      substituteInPlace $out/share/applications/${pname}.desktop \
        --replace 'Exec=AppRun' 'Exec=${pname}'
      cp -r ${contents}/usr/share/icons $out/share
    '';

  meta = {
    description = "Helium browser";
    homepage = "https://github.com/imputnet/helium-linux";
    mainProgram = "helium";
    platforms = [ "x86_64-linux" ];
  };
}
