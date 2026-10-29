{
  lib,
  craneLib,
  pkgs,
}:
let
  runtimeLibs = with pkgs; [
    vulkan-loader
    wayland
    libxkbcommon
  ];
  src = craneLib.cleanCargoSource ./.;
  lucide = pkgs.fetchzip {
    name = "lucide-static-1.64.0";
    url = "https://registry.npmjs.org/lucide-static/-/lucide-static-1.64.0.tgz";
    hash = "sha256-ObvLERXG5YwBzFHwdvJ+fKtch18n0pn8EZBQiX7YOV8=";
  };
  cratePatches = {
    gpui-pre = [ ./patches/gpui-pre-set-layer.patch ];
    gpui-pre-linux = [ ./patches/gpui-pre-linux-set-layer.patch ];
    gpui-pre-wgpu = [ ./patches/gpui-pre-wgpu-drop-shadow-clip.patch ];
  };
in
craneLib.buildPackage rec {
  inherit src;
  strictDeps = true;

  cargoVendorDir = craneLib.vendorCargoDeps {
    inherit src;
    overrideVendorCargoPackage =
      p: drv:
      if cratePatches ? ${p.name} then
        drv.overrideAttrs (old: {
          patches = (old.patches or [ ]) ++ cratePatches.${p.name};
        })
      else
        drv;
  };

  ISLAND_PCI_IDS = "${pkgs.hwdata}/share/hwdata/pci.ids";
  ISLAND_GLYPHS = "${lucide}/icons";

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

  postInstall = ''
    installShellCompletion --cmd island \
      --bash <($out/bin/island completions bash) \
      --fish <($out/bin/island completions fish) \
      --zsh <($out/bin/island completions zsh)
    $out/bin/island ai-hook plugin $out/share/island/claude-code
  '';

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit
      src
      strictDeps
      cargoVendorDir
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
    description = "Dynamic island status bar and notification daemon built with GPUI";
    platforms = lib.platforms.linux;
    mainProgram = "island";
  };
}
