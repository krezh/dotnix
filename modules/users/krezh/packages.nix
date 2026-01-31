{ inputs, ... }:
{
  flake.modules.homeManager.krezh =
    { pkgs, ... }:
    {
      home.packages =
        with pkgs;
        [
          curl
          ripgrep
          gh
          go
          go-task
          opentofu
          tofu-ls
          ncdu
          fd
          timer
          ffmpeg
          gowall
          await
          ntfy-sh
          hwatch
          gopls
          btop
          retry
          just
          minijinja
          gh-poi
          unzip
          shellcheck
          gum
          duf
          isd
          cava
          glow
          rust-analyzer
          hyperfine
          lefthook
          rclone
          wakatime-cli
          infisical

          # Networking
          speedtest-cli
          curlie
          doggo
          dig

          # Secrets
          age-plugin-yubikey
          sops
          age

          # Processors
          jq
          jc
          jnv
          yq-go
          diffsitter
          dyff
        ]
        ++ [
          # Add treefmt to prevent GC
          inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.treefmt
        ];
    };
}
