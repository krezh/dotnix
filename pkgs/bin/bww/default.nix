{
  lib,
  buildGoApplication,
  go-bin,
  makeWrapper,
  bitwarden-cli,
  walker,
  wl-clipboard,
  libnotify,
  pinentry-gnome3,
  libsecret,
}:
buildGoApplication rec {
  pname = "bww";
  version = "0.1.0";

  src = builtins.path {
    path = ./src;
    name = "bww-src";
  };

  go = go-bin.latestStable;
  modules = "${src}/govendor.toml";

  nativeBuildInputs = [makeWrapper];

  ldflags = [
    "-s"
    "-w"
  ];

  postInstall = ''
    wrapProgram $out/bin/${pname} \
      --prefix PATH : ${
      lib.makeBinPath [
        bitwarden-cli
        walker
        wl-clipboard
        libnotify
        pinentry-gnome3
        libsecret
      ]
    }
  '';

  meta = with lib; {
    description = "A Bitwarden menu interface using Walker";
    license = licenses.mit;
    mainProgram = pname;
  };
}
