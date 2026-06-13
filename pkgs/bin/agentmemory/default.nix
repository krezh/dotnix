{
  lib,
  stdenvNoCC,
  fetchurl,
  gnutar,
}:

stdenvNoCC.mkDerivation (finalAttrs: {
  pname = "agentmemory";
  # renovate: datasource=npm depName=@agentmemory/agentmemory
  version = "0.9.27";

  src = fetchurl {
    url = "https://registry.npmjs.org/@agentmemory/agentmemory/-/agentmemory-${finalAttrs.version}.tgz";
    hash = "sha256-m5pgNaGo6+MEuvkrscWOIzfyRl147mO+vDZHU8D7KiU=";
  };

  dontUnpack = true;

  installPhase = ''
    mkdir -p $out/scripts $out/dist $out/plugins/opencode/commands
    ${gnutar}/bin/tar -xzf $src package/plugin/scripts/ package/plugin/opencode/
    cp ./package/plugin/scripts/*.mjs $out/scripts/
    cp ./package/plugin/opencode/agentmemory-capture.ts $out/plugins/opencode/
    cp ./package/plugin/opencode/commands/*.md $out/plugins/opencode/commands/
    ${gnutar}/bin/tar -xzf $src --wildcards 'package/dist/standalone*.mjs'
    cp ./package/dist/standalone*.mjs $out/dist/
  '';

  meta = {
    description = "Persistent memory for AI coding agents";
    homepage = "https://github.com/rohitg00/agentmemory";
    license = lib.licenses.asl20;
  };
})
