{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go,
}:
buildGoModule rec {
  pname = "hyprmon";
  version = "0.0.11";
  src = fetchFromGitHub {
    owner = "erans";
    repo = "hyprmon";
    rev = "v${version}";
    hash = "sha256-TuxBdN8sjj0lH4DCPtj83HI9FlBaqAmPlpttnKuf+9Y=";
  };
  vendorHash = "sha256-sD+zpHg7hrsmosledXJ17bdFk+dSVTYitzJ7RuYJAIQ=";

  # Automatically patch go.mod to use the available Go version
  postPatch = ''
    GO_VERSION=$(${go}/bin/go version | grep -oP 'go\K[0-9]+\.[0-9]+(\.[0-9]+)?')
    sed -i "s|^go [0-9]\+\.[0-9]\+.*|go $GO_VERSION|" go.mod
  '';

  ldflags = [
    "-s"
    "-w"
  ];
  meta = {
    description = "TUI monitor configuration tool for Hyprland with visual layout, drag-and-drop, and profile management";
    homepage = "https://github.com/erans/hyprmon/";
    license = lib.licenses.asl20;
    mainProgram = "hyprmon";
  };
}
