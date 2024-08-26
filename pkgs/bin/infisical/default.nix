{
  stdenv,
  lib,
  fetchurl,
  installShellFiles,
}:

let
  # renovate: datasource=github-tag depName=infisical/infisical
  tagName = "infisical-cli/v0.28.5";

  version = lib.strings.removePrefix "infisical-cli/v" tagName;

  src =
    let
      suffix =
        {
          x86_64-linux = "linux_amd64";
          x86_64-darwin = "darwin_amd64";
          aarch64-linux = "linux_arm64";
          aarch64-darwin = "darwin_arm64";
        }
        ."${stdenv.hostPlatform.system}" or (throw "Unsupported system: ${stdenv.hostPlatform.system}");

      name = "infisical_${version}_${suffix}.tar.gz";
      hash = "sha256-hYDBZ6cTUCJTd5Zv/jCXVLpSGSeXbtC0uxAjh4NIjV8=";
      url = "https://github.com/Infisical/infisical/releases/download/infisical-cli%2Fv${version}/${name}";
    in
    fetchurl { inherit name url hash; };

in
stdenv.mkDerivation {
  pname = "infisical";
  version = version;
  inherit src;

  nativeBuildInputs = [ installShellFiles ];

  doCheck = true;
  dontConfigure = true;
  dontStrip = true;

  sourceRoot = ".";
  buildPhase = "chmod +x ./infisical";
  checkPhase = "./infisical --version";
  installPhase = ''
    mkdir -p $out/bin/ $out/share/completions/ $out/share/man/
    cp infisical $out/bin
    cp completions/* $out/share/completions/
    cp manpages/* $out/share/man/
  '';
  postInstall = ''
    installManPage share/man/infisical.1.gz
    installShellCompletion share/completions/infisical.{bash,fish,zsh}
  '';

  meta = with lib; {
    description = "Official Infisical CLI";
    longDescription = ''
      Infisical is the open-source secret management platform:
      Sync secrets across your team/infrastructure and prevent secret leaks.
    '';
    homepage = "https://infisical.com";
    changelog = "https://github.com/infisical/infisical/releases/tag/infisical-cli%2Fv${version}";
    license = licenses.mit;
    mainProgram = "infisical";
    maintainers = [ ];
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
      "x86_64-darwin"
    ];
  };
}
