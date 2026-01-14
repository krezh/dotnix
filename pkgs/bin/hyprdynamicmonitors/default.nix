{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "hyprdynamicmonitors";
  version = "1.4.0";

  src = fetchFromGitHub {
    owner = "fiffeek";
    repo = "hyprdynamicmonitors";
    rev = "v${version}";
    hash = "sha256-msAgix63TsGgETwJajdr//F19+UUhGCbrjinNbgMPHo=";
  };

  vendorHash = "sha256-WBK1PhhxaRa0FUAfSxtKOiesw71wy0753FYIgSlo0bE=";

  doCheck = false;

  ldflags = [
    "-s"
    "-w"
    "-X=github.com/fiffeek/hyprdynamicmonitors/cmd.Version=${version}"
    "-X=github.com/fiffeek/hyprdynamicmonitors/cmd.Commit=${src.rev}"
    "-X=github.com/fiffeek/hyprdynamicmonitors/cmd.BuildDate=1970-01-01T00:00:00Z"
  ];

  meta = {
    description = "In short: Autorandr+Arandr for Hyprland. Manage Hyprland configuration based on connected displays, power and lid state";
    homepage = "https://github.com/fiffeek/hyprdynamicmonitors";
    license = lib.licenses.mit;
    mainProgram = "hyprdynamicmonitors";
  };
}
