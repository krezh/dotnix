{ pkgs, lib, ... }:
pkgs.buildGoApplication {
  pname = "km";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./src;
    fileset = ./src;
  };
  modules = ./src/gomod2nix.toml;
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
}
