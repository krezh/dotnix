{ pkgs, ... }:
{
  home.packages = with pkgs; [
    curl
    ripgrep
    gh
    go
    dyff
    go-task
    opentofu
    ncdu
    fd
    httpie
    diffsitter
    timer
    ffmpeg
    yt-dlp
    gowall
    await
    ntfy-sh
    hwatch
    envsubst
    gopls
    tldr
    sd
    btop
    flyctl
    retry
    just
    minijinja
    gh-poi
    pre-commit
    p7zip
    unzip
    shellcheck
    gum
    duf
    isd
    doggo
    dig
    wowup-cf
    lazysql
    cava
    glow
    rust-analyzer
    hyperfine
    antares
    hypr-slurp
    vdhcoapp

    # Secrets
    age-plugin-yubikey
    yubikey-manager
    sops
    age
    doppler

    # Processors
    jq
    jc
    jnv
    yq-go
  ];

  hmModules.shell.aria2.enable = true;
  hmModules.shell.kubernetes.enable = true;
}
