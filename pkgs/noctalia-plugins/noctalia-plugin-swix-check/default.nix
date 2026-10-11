{
  lib,
  stdenvNoCC,
}:
stdenvNoCC.mkDerivation {
  pname = "noctalia-plugin-swix-check";
  version = "0.1.0";

  src = ./.;

  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p "$out"
    cp -r catalog.toml swix-check "$out/"
    runHook postInstall
  '';

  meta = {
    description = "Noctalia plugin showing incoming flake changes reported by swix check";
    homepage = "https://github.com/krezh/dotnix";
    license = lib.licenses.mit;
    platforms = lib.platforms.all;
  };
}
