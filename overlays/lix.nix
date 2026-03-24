_final: prev: {
  inherit (prev.lixPackageSets.latest)
    # nixpkgs-review
    # nix-fast-build
    nix-eval-jobs
    colmena
    ;
}
