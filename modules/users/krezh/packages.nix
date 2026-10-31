{
  flake.modules.homeManager.krezh =
    { pkgs, ... }:
    {
      home.packages = with pkgs; [
        curl
        ripgrep
        ncdu
        fd
        timer
        ffmpeg
        gowall
        await
        hwatch
        btop
        retry
        minijinja
        unzip
        unrar
        gum
        duf
        isd
        cava
        glow
        hyperfine
        infisical
        hcloud
        hister

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
        dyff
      ];
    };
}
