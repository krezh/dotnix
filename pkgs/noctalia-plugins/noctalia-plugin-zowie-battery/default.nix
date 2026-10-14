{
  lib,
  stdenvNoCC,
}:
stdenvNoCC.mkDerivation {
  pname = "noctalia-plugin-zowie-battery";
  version = "0.1.0";

  src = ./.;

  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p "$out"
    cp -r catalog.toml zowie-battery "$out/"
    runHook postInstall
  '';

  meta = {
    description = "Noctalia plugin showing the battery of a BenQ ZOWIE DW wireless mouse";
    homepage = "https://github.com/krezh/dotnix";
    license = lib.licenses.mit;
    platforms = lib.platforms.all;
  };
}
