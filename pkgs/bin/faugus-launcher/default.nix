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
  python3Packages,
  umu-launcher,
  lsfg-vk,
  wrapGAppsHook3,
  xdg-utils,
}:

python3Packages.buildPythonApplication (finalAttrs: {
  pname = "faugus-launcher";
  # renovate: datasource=github-releases depName=Faugus/faugus-launcher
  version = "1.22.4";
  pyproject = false;

  src = fetchFromGitHub {
    owner = "Faugus";
    repo = "faugus-launcher";
    tag = finalAttrs.version;
    hash = "sha256-Npfoqa6A1YSNSxV3zcIQL6prlht47dVaZYpq9+Dx9LY=";
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
    pillow
    psutil
    pygobject3
    requests
    vdf
  ];

  postPatch = ''
    substituteInPlace faugus-launcher \
      --replace-fail "/usr/bin/python3" "${python3Packages.python.interpreter}"

    substituteInPlace faugus/path_manager.py \
      --replace-fail "PathManager.user_data('faugus-launcher/umu-run')" "'${lib.getExe umu-launcher}'"

    substituteInPlace faugus/launcher.py faugus/shortcut.py \
      --replace-fail 'next((p for p in lsfgvk_possible_paths if p.exists()), lsfgvk_possible_paths[-1])' 'Path("${lsfg-vk}/lib/liblsfg-vk.so")'
  '';

  dontWrapGApps = true;

  preFixup =
    let
      pythonPath =
        with python3Packages;
        makePythonPath [
          pillow
          psutil
          pygobject3
          requests
          vdf
        ];
    in
    ''
      # Wrap faugus-launcher manually
      gappsWrapperArgs+=(
        --prefix PYTHONPATH : "$out/${python3Packages.python.sitePackages}:${pythonPath}"
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
      wrapProgram $out/bin/faugus-launcher "''${gappsWrapperArgs[@]}"

      # Set wrapper args for faugus-run
      makeWrapperArgs+=(
        "''${gappsWrapperArgs[@]}"
      )
    '';

  meta = {
    description = "Simple and lightweight app for running Windows games using UMU-Launcher";
    homepage = "https://github.com/Faugus/faugus-launcher";
    changelog = "https://github.com/Faugus/faugus-launcher/releases/tag/${finalAttrs.src.tag}";
    license = lib.licenses.mit;
    mainProgram = "faugus-launcher";
    platforms = lib.platforms.linux;
  };
})
