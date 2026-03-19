{
  rustPlatform,
  pkg-config,
  pkgs,
  wrapGAppsHook4,
  ...
}:
rustPlatform.buildRustPackage {
  pname = "optiman";
  version = "0.1.0";

  src = builtins.path {
    path = ./.;
    name = "optiman-src";
  };

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    wrapGAppsHook4
  ];

  buildInputs = [
    pkgs.gtk4
    pkgs.libadwaita
    pkgs.openssl
  ];

  meta = {
    description = "GTK4 manager for OptiScaler upscaling mod for Steam games";
    longDescription = ''
      OptiMan is a graphical application for managing OptiScaler installations
      across Steam games on Linux. It automatically detects Steam games,
      downloads OptiScaler releases, and provides a user-friendly interface
      for installation and configuration.
    '';
    homepage = "https://github.com/optiscaler/OptiScaler";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
    mainProgram = "optiman";
  };
}
