# Shell for bootstrapping flake-enabled nix and other tooling
{
  pkgs ? (import ./nixpkgs.nix) { },
  ...
}:
{
  default = pkgs.mkShell {
    NIX_CONFIG = "extra-experimental-features = nix-command flakes repl-flake";
    nativeBuildInputs = with pkgs; [
      nix
      home-manager
      git
      go-task
      sops
      ssh-to-age
      gnupg
      age
    ];
  };
}
