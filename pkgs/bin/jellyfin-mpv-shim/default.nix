{
  fetchFromGitHub,
  gettext,
  gobject-introspection,
  lib,
  python3Packages,
  wrapGAppsHook3,
}:

let
  shaderPack = fetchFromGitHub {
    owner = "iwalton3";
    repo = "default-shader-pack";
    tag = "v3.0.0";
    hash = "sha256-lHFidCHBduvNBy1HGgqLDqZMJeLv3jfVWQ73Hlev7w8=";
  };
in
python3Packages.buildPythonApplication (finalAttrs: {
  pname = "jellyfin-mpv-shim";
  # renovate: datasource=github-releases depName=jellyfin/jellyfin-mpv-shim
  version = "3.0.0pre9";
  pyproject = true;

  src = fetchFromGitHub {
    owner = "jellyfin";
    repo = "jellyfin-mpv-shim";
    tag = "v${finalAttrs.version}";
    hash = "sha256-wG3mxzT/5wo2bHo4MhPLMNH7k6lg97EawsQ4EBHAz6k=";
  };

  nativeBuildInputs = [
    gettext
    gobject-introspection
    wrapGAppsHook3
  ];

  build-system = with python3Packages; [ setuptools ];

  dependencies = with python3Packages; [
    jellyfin-apiclient-python
    mpv
    python-mpv-jsonipc
    requests
    pillow
    pystray
    pypresence
  ];

  postPatch = ''
    cp -r ${shaderPack} jellyfin_mpv_shim/default_shader_pack
    for file in jellyfin_mpv_shim/messages/*/LC_MESSAGES/base.po; do
      msgfmt "$file" -o "''${file%.po}.mo"
    done

    substituteInPlace jellyfin_mpv_shim/conf.py \
      --replace-fail "check_updates: bool = True" "check_updates: bool = False" \
      --replace-fail "notify_updates: bool = True" "notify_updates: bool = False"
    substituteInPlace jellyfin_mpv_shim/constants.py \
      --replace-fail 'CLIENT_VERSION = "2.10.0"' 'CLIENT_VERSION = "${finalAttrs.version}"'
    substituteInPlace jellyfin_mpv_shim/integration/com.github.iwalton3.jellyfin-mpv-shim.appdata.xml \
      --replace-fail '<release version="2.10.0" date="2026-05-03">' \
        '<release version="${finalAttrs.version}" date="2026-07-24">'
    substituteInPlace pyproject.toml \
      --replace-fail "python-mpv" "mpv" \
      --replace-fail "mpv-jsonipc" "python_mpv_jsonipc"
  '';

  preCheck = ''
    export HOME=$TMPDIR
    rm jellyfin_mpv_shim/win_utils.py
  '';

  postInstall = ''
    desktopId=com.github.iwalton3.jellyfin-mpv-shim
    install -Dm444 jellyfin_mpv_shim/integration/$desktopId.desktop \
      $out/share/applications/$desktopId.desktop
    install -Dm444 jellyfin_mpv_shim/integration/$desktopId.appdata.xml \
      $out/share/metainfo/$desktopId.appdata.xml

    for size in 16 32 48 64 128 256; do
      install -Dm444 jellyfin_mpv_shim/integration/jellyfin-$size.png \
        $out/share/icons/hicolor/''${size}x''${size}/apps/$desktopId.png
    done
  '';

  dontWrapGApps = true;
  preFixup = ''
    makeWrapperArgs+=("''${gappsWrapperArgs[@]}")
  '';

  pythonImportsCheck = [ "jellyfin_mpv_shim" ];

  meta = {
    description = "Cast media from Jellyfin Mobile and Web apps to MPV";
    homepage = "https://github.com/jellyfin/jellyfin-mpv-shim";
    changelog = "https://github.com/jellyfin/jellyfin-mpv-shim/releases/tag/v${finalAttrs.version}";
    license = with lib.licenses; [
      gpl3Plus
      mit
      unlicense
    ];
    mainProgram = "jellyfin-mpv-shim";
    platforms = lib.platforms.linux;
  };
})
