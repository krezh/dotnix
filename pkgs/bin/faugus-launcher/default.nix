{
  fetchFromGitHub,
  gobject-introspection,
  icoextract,
  imagemagick,
  lib,
  libayatana-appindicator,
  libcanberra-gtk3,
  meson,
  ninja,
  nix-update-script,
  python3Packages,
  umu-launcher,
  wrapGAppsHook3,
  xdg-utils,
}:
python3Packages.buildPythonApplication rec {
  pname = "faugus-launcher";
  # renovate: datasource=github-releases depName=Faugus/faugus-launcher
  version = "1.18.4";
  pyproject = false;

  src = fetchFromGitHub {
    owner = "Faugus";
    repo = "faugus-launcher";
    tag = version;
    hash = "sha256-861s4yCwJwKd0yVUKlTiHRTCdX4NkJt2vJlTh0pFPA4=";
  };

  nativeBuildInputs = [
    gobject-introspection
    meson
    ninja
    wrapGAppsHook3
  ];

  buildInputs = [
    libayatana-appindicator
  ];

  dependencies = with python3Packages; [
    filelock
    pillow
    psutil
    pygobject3
    requests
    vdf
  ];

  postPatch = ''
    substituteInPlace faugus/launcher.py faugus/runner.py faugus/shortcut.py \
      --replace-fail "PathManager.user_data('faugus-launcher/umu-run')" "'${lib.getExe umu-launcher}'"
  '';

  dontWrapGApps = true;

  preFixup = ''
    makeWrapperArgs+=(
      "''${gappsWrapperArgs[@]}"
      --suffix PATH : "${
        lib.makeBinPath [
          icoextract
          imagemagick
          libcanberra-gtk3
          umu-launcher
          xdg-utils
        ]
      }"
    )
  '';

  passthru.updateScript = nix-update-script { };

  meta = {
    description = "Simple and lightweight app for running Windows games using UMU-Launcher";
    homepage = "https://github.com/Faugus/faugus-launcher";
    changelog = "https://github.com/Faugus/faugus-launcher/releases/tag/${version}";
    license = lib.licenses.mit;
    mainProgram = "faugus-launcher";
    platforms = lib.platforms.linux;
  };
}
