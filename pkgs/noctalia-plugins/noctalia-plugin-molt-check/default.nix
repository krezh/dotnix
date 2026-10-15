{
  lib,
  stdenvNoCC,
}:
stdenvNoCC.mkDerivation {
  pname = "noctalia-plugin-molt-check";
  version = "0.1.0";

  src = ./.;

  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p "$out"
    cp -r catalog.toml molt-check "$out/"
    install -Dm644 ${../../bin/molt/data/io.github.krezh.Molt-symbolic.svg} "$out/molt-check/molt.svg"
    runHook postInstall
  '';

  meta = {
    description = "Noctalia plugin showing incoming flake changes reported by molt check";
    homepage = "https://github.com/krezh/dotnix";
    license = lib.licenses.mit;
    platforms = lib.platforms.all;
  };
}
