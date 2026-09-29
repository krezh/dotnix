{
  lib,
  stdenvNoCC,
}:
stdenvNoCC.mkDerivation {
  pname = "noctalia-plugin-omp-usage";
  version = "0.1.0";

  src = ./.;

  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p "$out"
    cp -r catalog.toml omp-usage "$out/"
    runHook postInstall
  '';

  meta = {
    description = "Noctalia plugin for tracking OMP provider usage and reset countdowns";
    homepage = "https://github.com/krezh/dotnix";
    license = lib.licenses.mit;
    platforms = lib.platforms.all;
  };
}
