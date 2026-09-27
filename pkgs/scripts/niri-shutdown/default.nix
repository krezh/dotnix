{
  lib,
  gcc16Stdenv,
  writeShellApplication,
  hyprshutdown,
  makeWrapper,
  niri,
  python3,
  util-linux,
  ...
}:
let
  shutdownUi = hyprshutdown.overrideAttrs (old: {
    pname = "niri-shutdown-ui";
    postPatch = (old.postPatch or "") + ''
      substituteInPlace src/main.cpp \
        --replace-fail 'hyprshutdown v{}' 'niri-shutdown v{}' \
        --replace-fail 'Do not exit hyprland once apps close' 'Do not exit Niri once apps close' \
        --replace-fail 'after all apps and Hyprland shut down' 'after all apps and Niri shut down' \
        --replace-fail 'after Hyprland exits' 'after Niri exits'
      substituteInPlace src/ui/UI.cpp \
        --replace-fail 'appClass("hyprshutdown")' 'appClass("niri-shutdown")' \
        --replace-fail 'force quit Hyprland' 'force quit Niri'
    '';
    nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [ makeWrapper ];
    preFixup = (old.preFixup or "") + ''
      wrapProgram $out/bin/hyprshutdown \
        --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [ gcc16Stdenv.cc.cc.lib ]}
    '';
    meta = old.meta // {
      description = "A graceful shutdown utility for Niri";
    };
  });
in
writeShellApplication {
  name = "niri-shutdown";

  runtimeInputs = [
    niri
    python3
    util-linux
  ];

  text = ''
    export NIRI_SHUTDOWN_ADAPTER=${lib.escapeShellArg ./adapter.py}
    export NIRI_SHUTDOWN_UI=${lib.escapeShellArg (lib.getExe' shutdownUi "hyprshutdown")}
    ${builtins.readFile ./shutdown.sh}
  '';
}
