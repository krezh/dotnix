{
  lib,
  pkgs,
}:
pkgs.rustPlatform.buildRustPackage (finalAttrs: {
  pname = "herdr-app";
  # renovate: datasource=github-tags depName=herdrdev/herdr-app
  version = "0.1.15";
  # renovate: datasource=github-tags depName=herdrdev/herdr-core
  coreVersion = "0.1.20";

  __structuredAttrs = true;

  src = pkgs.fetchFromGitHub {
    owner = "herdrdev";
    repo = "herdr-app";
    tag = "v${finalAttrs.version}";
    fetchSubmodules = true;
    hash = "sha256-CNktD+tQrMMN4vpzFPZkZWnwDalaTkUV0A2UiLVeO3E=";
  };

  coreSrc = pkgs.fetchFromGitHub {
    owner = "herdrdev";
    repo = "herdr-core";
    tag = "v${finalAttrs.coreVersion}";
    fetchSubmodules = true;
    hash = "sha256-YzKVuMPt8CPns3q+1gnD6AcoDhuuqeh5zNMOq79aZ0o=";
  };

  cargoDeps = pkgs.symlinkJoin {
    name = "herdr-app-${finalAttrs.version}-vendor";
    paths = [
      (pkgs.rustPlatform.fetchCargoVendor {
        inherit (finalAttrs) src;
        name = "herdr-app-${finalAttrs.version}-vendor";
        hash = "sha256-usCdairGam65XVKtb1jJpRC0tNXnoafAnMFQjPgvgqQ=";
      })
      (pkgs.rustPlatform.fetchCargoVendor {
        src = finalAttrs.coreSrc;
        name = "herdr-core-${finalAttrs.coreVersion}-vendor";
        hash = "sha256-v+0gGNHJX1tWTBBg4F5kUeu7QHHG9A+0ZagvpiIeQeE=";
      })
    ];
  };

  zigDeps = pkgs.buildPackages.zig_0_16.fetchDeps {
    pname = "herdr-app";
    inherit (finalAttrs) version;
    src = "${finalAttrs.src}/vendor/ghostty";
    fetchAll = true;
    hash = "sha256-G9xbZ/pyQTDIfzG0h++i70a45zpwyYpZMBZfAnoOQw0=";
  };

  nativeBuildInputs = [
    pkgs.buildPackages.zig_0_16.hook
    pkgs.rustPlatform.bindgenHook
    pkgs.clang
    pkgs.pkg-config
    pkgs.makeWrapper
    pkgs.installAgentSkills
  ];

  buildInputs = [
    pkgs.fontconfig
    pkgs.freetype
    pkgs.libxkbcommon
    pkgs.libxcb
    pkgs.wayland
  ];

  env = {
    ZIG = lib.getExe pkgs.buildPackages.zig_0_16;
    NIX_SET_BUILD_ID = "1";
    NIX_LDFLAGS = "--strip-debug";
  };

  doCheck = false;

  dontUseZigBuild = true;
  dontUseZigCheck = true;
  dontUseZigInstall = true;
  dontInstallAgentSkills = true;

  postUnpack = ''
    cp -r ${finalAttrs.coreSrc} $sourceRoot/herdr-core
    chmod -R u+w $sourceRoot/herdr-core
  '';

  postPatch = ''
    substituteInPlace herdr-core/vendor/ghostty/src/build/GhosttyLibVt.zig \
      --replace-fail 'lib.bundle_compiler_rt = true;' 'lib.bundle_compiler_rt = false;' \
      --replace-fail 'lib.bundle_ubsan_rt = true;' 'lib.bundle_ubsan_rt = false;'

    substituteInPlace herdr-core/crates/herdr-core/src/releases.rs \
      --replace-fail 'Command::new("curl")' 'Command::new("${lib.getExe pkgs.curl}")' \
      --replace-fail 'run("sha256sum"' 'run("${lib.getExe' pkgs.coreutils "sha256sum"}"' \
      --replace-fail '_ => "tar".into()' '_ => "${lib.getExe' pkgs.libarchive "bsdtar"}".into()'

    substituteInPlace herdr-core/crates/herdr-core/src/machines.rs \
      --replace-fail 'let bin = if there == here && (' 'let bin = if false && ('
  '';

  postConfigure = ''
    export ZIG_GLOBAL_CACHE_DIR=$(mktemp -d)
    cp -rL ${finalAttrs.zigDeps} "$ZIG_GLOBAL_CACHE_DIR/p"
    chmod -R u+w "$ZIG_GLOBAL_CACHE_DIR/p"
  '';

  postBuild = ''
    cargo build --offline --release -j $NIX_BUILD_CORES \
      --manifest-path herdr-core/Cargo.toml -p herdr-core
  '';

  postInstall = ''
    install -Dm755 herdr-core/target/release/herdr-core -t $out/bin
    ln -s herdr-core $out/bin/hc
    installSkill herdr-core/skills/herdr-core
    install -Dm444 assets/icons/herdr-ui-icon-clean.svg $out/share/icons/hicolor/scalable/apps/herdr-app.svg
    install -Dm444 /dev/stdin $out/share/applications/herdr-app.desktop <<EOF
    [Desktop Entry]
    Type=Application
    Name=Herdr
    Comment=Terminals and coding agents that keep running
    Exec=herdr-app
    Icon=herdr-app
    Terminal=false
    Categories=Development;
    StartupWMClass=herdr-app
    EOF
  '';

  postFixup = ''
    patchelf $out/bin/herdr-app --add-rpath ${
      lib.makeLibraryPath [
        pkgs.vulkan-loader
        pkgs.wayland
        pkgs.libxkbcommon
      ]
    }
    wrapProgram $out/bin/herdr-app --suffix PATH : $out/bin
  '';

  meta = {
    description = "Herdr desktop app for terminals and coding agents, with herdr-core";
    homepage = "https://github.com/herdrdev/herdr-app";
    changelog = "https://github.com/herdrdev/herdr-app/blob/v${finalAttrs.version}/CHANGELOG.md";
    license = with lib.licenses; [
      asl20
      mit
    ];
    sourceProvenance = [ lib.sourceTypes.fromSource ];
    platforms = lib.platforms.linux;
    mainProgram = "herdr-app";
  };
})
