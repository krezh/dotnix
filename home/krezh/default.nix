{
  inputs,
  outputs,
  lib,
  config,
  osConfig,
  pkgs,
  hostname,
  ...
}:
let
  isDesktop =
    !(builtins.elem hostname) [
      "thor-wsl"
      "steamdeck"
      "rpi-01"
      "rpi-02"
    ];
in
{
  imports = [
    (inputs.import-tree ./features/shell)
    inputs.nix-index.homeModules.nix-index
    inputs.catppuccin.homeModules.catppuccin
  ]
  ++ (if isDesktop then [ ./features/desktop ] else [ ])
  ++ outputs.homeManagerModules;

  programs.nix-index.enable = true;

  xdg.enable = true;

  catppuccin = {
    enable = true;
    flavor = "mocha";
    accent = "blue";
  };

  catppuccin.cursors.enable = true;
  catppuccin.cursors.flavor = "mocha";
  catppuccin.cursors.accent = "light";

  programs.nix-search-tv.enableTelevisionIntegration = true;
  programs.nix-search-tv.enable = true;
  programs.television.enable = true;
  programs.television.enableFishIntegration = true;

  sops = {
    age.keyFile = "${config.xdg.configHome}/sops/age/keys.txt";
    defaultSopsFile = ./secrets.sops.yaml;
    secrets = {
      "ssh/privkey" = {
        path = "/home/${config.home.username}/.ssh/id_ed25519";
        mode = "0600";
      };
      "atuin/key" = {
        path = "${config.xdg.configHome}/atuin/key";
      };
      "zipline/token" = {
        path = "${config.xdg.configHome}/zipline/token";
      };
      "kopia/password" = {
        path = "${config.xdg.configHome}/kopia/repository.password";
      };
    };
  };

  home = {
    homeDirectory = lib.mkDefault "/home/${config.home.username}";
    stateVersion = osConfig.system.stateVersion;
    preferXdgDirectories = true;
    sessionPath = [
      "$HOME/.local/bin"
      "$GOPATH/bin"
      "$CARGO_HOME/bin"
    ];
    sessionVariables = {
      FLAKE = "${config.home.homeDirectory}/nix-config";
      NH_FLAKE = "${config.home.homeDirectory}/nix-config";
      GOPATH = "${config.xdg.dataHome}/go";
      CARGO_HOME = "${config.xdg.dataHome}/cargo";
      SOPS_AGE_KEY_FILE = "${config.sops.age.keyFile}";
      SOPS_AGE_KEY_CMD = "age-plugin-yubikey --identity";
    };
    packages = with pkgs; [
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
  };

  hmModules.shell.aria2.enable = true;
  hmModules.shell.kubernetes.enable = true;

  programs = {
    home-manager.enable = true;
    yazi.enable = true;
  };

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";
}
