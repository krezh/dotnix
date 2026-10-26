{
  lib,
  stdenvNoCC,
  fetchFromGitHub,
  go-crx3,
}:

stdenvNoCC.mkDerivation (_finalAttrs: {
  pname = "compass";
  version = "0.1.1";

  src = fetchFromGitHub {
    owner = "justin22";
    repo = "compass";
    rev = "4674fbafa849aca94bb133c41b81a0b991d36f65";
    hash = "sha256-q96baRh+EwDC+z303j3o+sPRFQ5Y+vbpAGuI05FJIBU=";
  };

  nativeBuildInputs = [ go-crx3 ];

  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r * $out/
    crx3 pack . -p ${./key.pem} -o $out/compass.crx
    runHook postInstall
  '';

  passthru = {
    extId = "eikjlkhjhphpknjphmbbjjhjflfekdeb";
    crxPath = "${placeholder "out"}/compass.crx";
  };

  meta = {
    description = "A chrome extension for arc lovers";
    homepage = "https://github.com/justin22/compass";
    license = lib.licenses.mit;
    platforms = lib.platforms.all;
  };
})
