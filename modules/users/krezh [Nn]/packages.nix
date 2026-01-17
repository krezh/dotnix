{
  flake.modules.homeManager.krezh =
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
        tofu-ls
        ncdu
        fd
        httpie
        diffsitter
        timer
        ffmpeg
        gowall
        await
        ntfy-sh
        hwatch
        envsubst
        gopls
        tldr
        sd
        btop
        retry
        just
        minijinja
        gh-poi
        p7zip
        unzip
        shellcheck
        gum
        duf
        isd
        doggo
        dig
        lazysql
        cava
        glow
        rust-analyzer
        hyperfine
        curlie

        # Secrets
        age-plugin-yubikey
        sops
        age

        # Processors
        jq
        jc
        jnv
        yq-go

        # CLI tools
        speedtest-cli
        rclone
        wakatime-cli
      ];
    };
}
