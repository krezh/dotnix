{
  lib,
  buildGoModule,
}:

buildGoModule rec {
  pname = "recshot";
  version = "0.1.0";

  src = ./.;

  vendorHash = "sha256-eKeUhS2puz6ALb+cQKl7+DGvm9Cl+miZAHX0imf9wdg=";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = with lib; {
    description = "A tool for taking screenshots and recordings and uploading them to Zipline";
    license = licenses.mit;
    mainProgram = pname;
  };
}
