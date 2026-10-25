{
  lib,
  stdenvNoCC,
  fetchurl,
}:

let
  addonId = "share-capture@opentodo";
in
stdenvNoCC.mkDerivation (finalAttrs: {
  pname = "opentodo-browser-extension";
  # renovate: datasource=github-releases depName=pixari/OpenTodo
  version = "0.13.0";

  src = fetchurl {
    url = "https://github.com/pixari/OpenTodo/releases/download/v${finalAttrs.version}/opentodo-browser-extension-firefox.zip";
    hash = "sha256-/E8x6wSS1ehkWgp+ap9ExisNm7+GFu6HXJ5ss47ONe0=";
  };

  dontUnpack = true;

  installPhase = ''
    runHook preInstall
    dst="$out/share/mozilla/extensions/{ec8030f7-c20a-464f-9b0e-13a3a9e97384}"
    mkdir -p "$dst"
    install -v -m644 "$src" "$dst/${addonId}.xpi"
    ln -s "$dst/${addonId}.xpi" "$out/${addonId}.xpi"
    runHook postInstall
  '';

  passthru = {
    inherit addonId;
    extid = addonId;
  };

  meta = {
    description = "OpenTodo Capture browser extension for quick task capture";
    homepage = "https://github.com/pixari/OpenTodo/tree/main/extension";
    changelog = "https://github.com/pixari/OpenTodo/releases/tag/v${finalAttrs.version}";
    license = lib.licenses.agpl3Only;
    platforms = lib.platforms.all;
  };
})
