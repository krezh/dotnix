{
  lib,
  appimageTools,
  fetchurl,
}:
let
  pname = "jayjay";
  # renovate: datasource=github-releases depName=hewigovens/jayjay
  version = "0.3.22";
  src = fetchurl {
    url = "https://github.com/hewigovens/jayjay/releases/download/v${version}/jayjay-gpui-x86_64.AppImage";
    hash = "sha256-3U5cKxVwj6LXAXSGWoYv0rHYZL9ANC83/kxbNaL0bfY=";
  };

  appimageContents = appimageTools.extract { inherit pname version src; };
in
appimageTools.wrapType2 {
  inherit pname version src;

  extraInstallCommands = ''
    install -Dm444 ${appimageContents}/usr/share/applications/dev.hewig.JayJay.desktop -t $out/share/applications
    substituteInPlace $out/share/applications/dev.hewig.JayJay.desktop \
      --replace-fail 'Exec=jayjay-gpui' 'Exec=${pname}'
    cp -r ${appimageContents}/usr/share/icons $out/share/icons
  '';

  meta = {
    description = "Native GUI for the Jujutsu version control system";
    homepage = "https://github.com/hewigovens/jayjay";
    changelog = "https://github.com/hewigovens/jayjay/releases/tag/v${version}";
    license = lib.licenses.bsl11;
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
    platforms = [ "x86_64-linux" ];
    mainProgram = pname;
  };
}
