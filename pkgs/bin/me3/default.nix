{
  lib,
  stdenv,
  fetchurl,
  autoPatchelfHook,
  openssl,
  gcc-unwrapped,
  buildFHSEnv,
}:

let
  me3-unwrapped = stdenv.mkDerivation (finalAttrs: {
    pname = "me3-unwrapped";
    version = "0.10.1";

    src = fetchurl {
      url = "https://github.com/garyttierney/me3/releases/download/v${finalAttrs.version}/me3-linux-amd64.tar.gz";
      hash = "sha256-VhTuk0SxuAKrGEQlxewhlFP1znuJrj52zYo3VoTFAH0=";
    };

    sourceRoot = ".";

    nativeBuildInputs = [
      autoPatchelfHook
    ];

    buildInputs = [
      openssl
      gcc-unwrapped.lib
    ];

    installPhase = ''
      runHook preInstall

      install -Dm755 bin/me3 $out/bin/me3

      # Windows binaries needed for Proton-based game launching
      install -Dm755 bin/win64/me3-launcher.exe $out/share/me3/windows-bin/me3-launcher.exe
      install -Dm755 bin/win64/me3_mod_host.dll $out/share/me3/windows-bin/me3_mod_host.dll

      # Desktop integration
      install -Dm644 dist/me3-launch.desktop $out/share/applications/me3-launch.desktop
      install -Dm644 dist/me3.xml $out/share/mime/packages/me3.xml
      if [ -f dist/me3.png ]; then
        install -Dm644 dist/me3.png $out/share/icons/hicolor/128x128/apps/me3.png
      fi

      runHook postInstall
    '';
  });
in
buildFHSEnv {
  pname = "me3";
  inherit (me3-unwrapped) version;

  targetPkgs = pkgs: [
    me3-unwrapped
    pkgs.openssl
    pkgs.gcc-unwrapped.lib
    pkgs.libGL
  ];

  runScript = "me3";

  # Expose share/ so desktop/MIME/icon files are visible to the system
  extraInstallCommands = ''
    ln -s ${me3-unwrapped}/share $out/share
  '';

  meta = {
    description = "A framework for modding and instrumenting games";
    homepage = "https://github.com/garyttierney/me3";
    changelog = "https://github.com/garyttierney/me3/blob/v${me3-unwrapped.version}/CHANGELOG.md";
    license = with lib.licenses; [
      mit
      asl20
    ];
    mainProgram = "me3";
    platforms = [ "x86_64-linux" ];
  };
}
