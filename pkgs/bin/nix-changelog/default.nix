{
  lib,
  craneLib,
  installShellFiles,
}:
craneLib.buildPackage rec {
  pname = "nix-changelog";
  version = "0.1.0";

  src = craneLib.cleanCargoSource ./.;
  strictDeps = true;
  cargoArtifacts = craneLib.buildDepsOnly { inherit src strictDeps; };

  nativeBuildInputs = [
    installShellFiles
  ];

  postInstall = ''
    installShellCompletion --cmd nix-changelog \
      --bash <($out/bin/nix-changelog completion bash) \
      --fish <($out/bin/nix-changelog completion fish) \
      --zsh <($out/bin/nix-changelog completion zsh)
  '';

  meta = {
    description = "Pretty-print a package's changelog from a Nix flake reference";
    mainProgram = "nix-changelog";
    license = lib.licenses.mit;
    platforms = lib.platforms.unix;
  };
}
