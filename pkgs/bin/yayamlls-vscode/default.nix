{
  lib,
  buildNpmPackage,
  fetchFromGitHub,
  pkgs,
}:

buildNpmPackage (finalAttrs: {
  pname = "yayamlls-vscode";
  # renovate: datasource=github-releases depName=home-operations/yayamlls
  version = "0.3.1";

  src = fetchFromGitHub {
    owner = "home-operations";
    repo = "yayamlls";
    tag = finalAttrs.version;
    hash = "sha256-0WL7qC5gLIMkjwnZ3lD1tD91ky0A5wG84EphLWe2wWc=";
  };

  sourceRoot = "source/editors/vscode";

  npmDepsFetcherVersion = 2;
  npmDepsHash = "sha256-FSGq4mpS6c7I17XEGFcT50W/uTapo9eZDST+uZeDHhw=";

  nativeBuildInputs = [
    pkgs.vsce
    pkgs.unzip
    pkgs.jq
  ];

  postPatch = ''
    ${lib.getExe pkgs.jq} --arg ver "^${lib.versions.major pkgs.vscodium.version}.${lib.versions.minor pkgs.vscodium.version}.0" \
      '.engines.vscode = $ver | .devDependencies["@types/vscode"] = $ver' \
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
