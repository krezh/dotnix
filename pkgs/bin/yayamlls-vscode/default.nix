{
  lib,
  buildNpmPackage,
  fetchFromGitHub,
  pkgs,
}:

buildNpmPackage (finalAttrs: {
  pname = "yayamlls-vscode";
  # renovate: datasource=github-releases depName=home-operations/yayamlls
  version = "0.1.14";

  src = fetchFromGitHub {
    owner = "home-operations";
    repo = "yayamlls";
    tag = finalAttrs.version;
    hash = "sha256-Sw6nAohucCsO9tEq0/N98QK4PAtWsDE6RoKy3aNJ8j8=";
  };

  sourceRoot = "source/editors/vscode";

  npmDepsFetcherVersion = 2;
  npmDepsHash = "sha256-OpB1/ZodWzJTHhgogc4gctIBxenXKHI5j2EM3j7Dczk=";

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
