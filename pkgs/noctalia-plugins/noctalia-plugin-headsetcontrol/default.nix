{
  lib,
  stdenvNoCC,
}:
stdenvNoCC.mkDerivation {
  pname = "noctalia-plugin-headsetcontrol";
  version = "0.1.0";

  src = ./.;

  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p "$out"
    cp -r catalog.toml headsetcontrol "$out/"
    runHook postInstall
  '';

  meta = {
    description = "Noctalia plugin showing headset battery levels reported by HeadsetControl";
    homepage = "https://github.com/krezh/dotnix";
    license = lib.licenses.mit;
    platforms = lib.platforms.all;
  };
}
