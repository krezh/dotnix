{
  pkgs,
  go-bin,
  ...
}:
pkgs.buildGoApplication rec {
  pname = "km";
  version = "0.1.0";
  src = builtins.path {
    path = ./src;
    name = "kopia-manager-src";
  };

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
}
