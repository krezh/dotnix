{
  lib,
  buildGoModule,
}:

buildGoModule rec {
  pname = "recshot";
  version = "0.1.0";

  src = ./src;

  vendorHash = null;

  ldflags = [
    "-s"
    "-w"
  ];

  postInstall = ''
    # Install the icon
    install -Dm644 $src/recshot.png $out/share/pixmaps/recshot.png

    # Also install it relative to the binary for fallback
    install -Dm644 $src/recshot.png $out/share/recshot/recshot.png
  '';

  meta = with lib; {
    description = "A tool for taking screenshots and recordings and uploading them to Zipline";
    license = licenses.mit;
    mainProgram = pname;
  };
}
