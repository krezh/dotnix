{
  lib,
  buildNpmPackage,
  fetchFromGitHub,
  vscodium,
  pkgs,
}:

buildNpmPackage (finalAttrs: {
  pname = "yayamlls-vscode";
  # renovate: datasource=github-releases depName=home-operations/yayamlls
  version = "0.1.10";

  src = fetchFromGitHub {
    owner = "home-operations";
    repo = "yayamlls";
    tag = finalAttrs.version;
    hash = "sha256-E0loH//MdaXui/tLoiNoL+L1RBPgh0TlnF1M6flxKto=";
  };

  sourceRoot = "source/editors/vscode";

  npmDepsHash = "sha256-cSo8C+mbU9C3tP5EgNnsE06Ok+ZNtyGO9i1cSGQUwnQ=";

  nativeBuildInputs = [
    pkgs.vsce
    pkgs.unzip
    pkgs.jq
  ];

  postPatch = ''
    ${lib.getExe pkgs.jq} '.engines.vscode = "^${lib.versions.major vscodium.version}.${lib.versions.minor vscodium.version}.0"' \
      package.json > package.json.tmp
    mv package.json.tmp package.json
  '';

  buildPhase = "vsce package";

  installPhase = ''
    unzip -q yayamlls-*.vsix -d unpacked
    mkdir -p "$out/share/vscode/extensions/home-operations.yayamlls"
    cp -r unpacked/extension/. "$out/share/vscode/extensions/home-operations.yayamlls/"
  '';

  passthru = {
    vscodeExtPublisher = "home-operations";
    vscodeExtName = "yayamlls";
    vscodeExtUniqueId = "home-operations.yayamlls";
  };

  meta = {
    description = "YAML language server VS Code extension";
    homepage = "https://github.com/home-operations/yayamlls";
    license = lib.licenses.mit;
  };
})
