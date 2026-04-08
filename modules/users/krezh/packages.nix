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
          ncdu
          fd
          timer
          ffmpeg
          gowall
          await
          ntfy-sh
          hwatch
          btop
          retry
          minijinja
          gh-poi
          unzip
          gum
          duf
          isd
          cava
          glow
          hyperfine
          rclone
          wakatime-cli
          infisical

          # Dev
          devenv
          lefthook
          rust-analyzer
          shellcheck
          just
          gopls
          zizmor
          go
          go-task
          opentofu
          tofu-ls
          statix
          nixd
          nil

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
          dyff
        ]
        ++ [
          # Add treefmt to prevent GC
          inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.treefmt
        ];
    };
}
