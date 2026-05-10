{
  pkgs,
  go-bin,
  lib,
  ...
}:
let
  src = builtins.path {
    path = ./src;
    name = "kopia-manager-src";
  };
in
pkgs.buildGoApplication {
  pname = "km";
  version = "0.1.0";
  inherit src;

  go = go-bin.latestStable;
  modules = "${src}/govendor.toml";
  buildInputs = [ pkgs.kopia ];
  ldflags = [
    "-s"
    "-w"
  ];
  postInstall = ''
    # Rename the binary from kopia-manager to km
    mv $out/bin/kopia-manager $out/bin/km

    installShellCompletion --cmd km \
      --bash <($out/bin/km completion bash) \
      --zsh <($out/bin/km completion zsh) \
      --fish <($out/bin/km completion fish)
  '';
  nativeBuildInputs = with pkgs; [ installShellFiles ];

  meta = {
    description = "Manages Kopia repository operations, snapshots, and systemd backup services";
    homepage = "https://github.com/krezh/dotnix";
    license = lib.licenses.mit;
    mainProgram = "km";
  };
}
