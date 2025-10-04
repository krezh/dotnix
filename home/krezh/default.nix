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
    ./features/shell
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
  catppuccin.hyprland.enable = true;
  catppuccin.gtk.icon.enable = true;
  catppuccin.vesktop.enable = true;
  catppuccin.vscode.profiles.default.enable = false;
  catppuccin.k9s.enable = true;
  catppuccin.k9s.transparent = true;

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
      unstable.go
      dyff
      go-task
      unstable.opentofu
      ncdu
      fd
      httpie
      diffsitter
      timer
      bottom
      ffmpeg
      yt-dlp
      gowall
      await
      ntfy-sh
      procs
      hwatch
      envsubst
      gopls
      tldr
      sd
      btop
      flyctl
      retry
      just
      terraform
      minijinja
      gh-poi
      pre-commit
      p7zip
      unzip
      shellcheck
      gum
      cava
      duf
      systemctl-tui
      isd
      doggo
      dig
      wowup-cf
      lazysql

      # Secrets
      age-plugin-yubikey
      yubikey-manager
      sops
      age
      doppler
      infisical

      # Processors
      jq
      jc
      jnv
      yq-go

      # Nix
      cachix
      nixfmt-rfc-style
      nvd
      nix-output-monitor
      niv
      comma
      nix-tree
      nixos-anywhere
      nixos-shell
      any-nix-shell
      attic-client

      # Kubernetes
      talosctl
      unstable.kubectl
      kubeswitch
      kubectl-cnpg
      kubectl-node-shell
      kubectl-klock
      kubectl-df-pv
      unstable.kubecolor
      kustomize
      fluxcd
      stern
      helmfile
      kubernetes-helm
      kubernetes-helmPlugins.helm-diff
      kind
      unstable.teleport
      kubestr
      kubectl-pgo
      cilium-cli
      kubectl-rook-ceph
    ];
  };

  hmModules.shell.krew.enable = true;
  hmModules.shell.kubectx.enable = true;
  hmModules.shell.aria2.enable = true;
  hmModules.shell.talswitcher.enable = true;

  programs = {
    home-manager.enable = true;
    neomutt.enable = true;
    yazi.enable = true;
  };

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";
}
