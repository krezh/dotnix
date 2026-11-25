{
  lib,
  pkgs,
}:
let
  runtimeLibs = with pkgs; [
    vulkan-loader
    wayland
    libxkbcommon
  ];
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      (lib.fileset.fromSource (pkgs.craneLib.cleanCargoSource ./.))
      (pkgs.craneLib.fileset.commonCargoSources ../nix-changelog)
    ];
  };
  postUnpack = ''
    cd "$sourceRoot/island"
    sourceRoot="."
  '';
  lucide = pkgs.fetchzip {
    name = "lucide-static-1.64.0";
    url = "https://registry.npmjs.org/lucide-static/-/lucide-static-1.64.0.tgz";
    hash = "sha256-ObvLERXG5YwBzFHwdvJ+fKtch18n0pn8EZBQiX7YOV8=";
  };
in
pkgs.craneLib.buildPackage rec {
  inherit src postUnpack;
  cargoToml = ./Cargo.toml;
  cargoLock = ./Cargo.lock;
  strictDeps = true;

  ISLAND_PCI_IDS = "${pkgs.hwdata}/share/hwdata/pci.ids";
  ISLAND_GLYPHS = "${lucide}/icons";
  MOLT_GIT = lib.getExe pkgs.git;
  MOLT_JJ = lib.getExe pkgs.jujutsu;

  # SQLCipher, bundled by rusqlite, encrypts with OpenSSL's libcrypto.
  OPENSSL_LIB_DIR = "${lib.getLib pkgs.openssl}/lib";
  OPENSSL_INCLUDE_DIR = "${lib.getDev pkgs.openssl}/include";

  buildInputs = with pkgs; [
    fontconfig
    freetype
    libpulseaudio
    libxkbcommon
    openssl
    udev
    wayland
  ];

  nativeBuildInputs = with pkgs; [
    pkg-config
    installShellFiles
  ];

  nativeCheckInputs = with pkgs; [ git ];

  preCheck = ''
    export HOME="$TMPDIR/home"
    export XDG_CACHE_HOME="$HOME/.cache"
    export XDG_CONFIG_HOME="$HOME/.config"
    mkdir -p "$XDG_CACHE_HOME" "$XDG_CONFIG_HOME"
  '';
  cargoTestExtraArgs = "--workspace";

  postInstall = ''
    installShellCompletion --cmd island \
      --bash <($out/bin/island completions bash) \
      --fish <($out/bin/island completions fish) \
      --zsh <($out/bin/island completions zsh)
    $out/bin/island ai-hook plugin $out/share/island/claude-code
  '';

  cargoArtifacts = pkgs.craneLib.buildDepsOnly {
    inherit
      src
      postUnpack
      cargoToml
      cargoLock
      strictDeps
      buildInputs
      nativeBuildInputs
      OPENSSL_LIB_DIR
      OPENSSL_INCLUDE_DIR
      ;
  };

  postFixup = ''
    patchelf --add-rpath ${lib.makeLibraryPath runtimeLibs} $out/bin/island
  '';

  meta = {
    description = "Dynamic island status bar and notification daemon drawn with wgpu";
    platforms = lib.platforms.linux;
    mainProgram = "island";
  };
}
