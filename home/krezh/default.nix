{
  inputs,
  outputs,
  lib,
  config,
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
  imports =
    [
      ./features/shell
      inputs.nix-index.hmModules.nix-index
      inputs.catppuccin.homeModules.catppuccin
    ]
    ++ (if isDesktop then [ ./features/desktop ] else [ ])
    ++ outputs.homeManagerModules;

  nixpkgs = {
    overlays = builtins.attrValues outputs.overlays;
  };

  programs.nix-index.enable = true;

  xdg.enable = true;

  catppuccin = {
    enable = true;
    flavor = "mocha";
    accent = "lavender";
  };

  sops = {
    age.keyFile = "${config.xdg.configHome}/sops/age/keys.txt";
    defaultSopsFile = ./secrets.sops.yaml;
    gnupg.sshKeyPaths = [ ];
    secrets = {
      "ssh/privkey" = {
        path = "/home/${config.home.username}/.ssh/id_ed25519";
        mode = "0600";
      };
      "atuin/key" = {
        path = "${config.xdg.configHome}/atuin/key";
      };
      "tlk/proxy" = {
        path = "${config.xdg.configHome}/tlk/proxy";
      };
    };
  };

  home = {
    homeDirectory = lib.mkDefault "/home/${config.home.username}";
    stateVersion = lib.mkDefault "24.05";
    preferXdgDirectories = true;
    sessionPath = [
      "$HOME/.local/bin"
      "$GOPATH/bin"
      "$CARGO_HOME/bin"
    ];
    sessionVariables = {
      FLAKE = "${config.home.homeDirectory}/nix-config";
      GOPATH = "${config.xdg.dataHome}/go";
      CARGO_HOME = "${config.xdg.dataHome}/cargo";
      SOPS_AGE_KEY_FILE = "${config.sops.age.keyFile}";
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
      bottom
      ffmpeg
      yt-dlp
      earthly
      gowall
      await
      ntfy-sh
      procs
      hwatch
      envsubst
      gopls
      tldr
      sd
      act
      see
      btop
      flyctl
      up
      retry
      cue
      minio-client
      just
      terraform
      minijinja
      gh-poi
      xxd

      # Secrets
      age-plugin-yubikey
      yubikey-manager
      sops
      age
      doppler
      infisical
      doggo

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
      tlk
      kubestr
      kubectl-pgo
      cilium-cli
      kubectl-rook-ceph
    ];
  };

  hmModules.shell.krew.enable = true;
  hmModules.shell.kubectx.enable = true;
  hmModules.shell.aria2.enable = true;

  programs = {
    home-manager.enable = true;
    neomutt.enable = true;
    yazi.enable = true;
  };

  nix = {
    settings = {
      accept-flake-config = true;
      always-allow-substitutes = true;
      experimental-features = [
        "nix-command"
        "flakes"
      ];
      trusted-users = [
        "root"
        "@wheel"
      ];
      builders-use-substitutes = true;
      warn-dirty = false;
      extra-substituters = [
        "https://cache.garnix.io"
        "https://krezh.cachix.org"
        "https://cache.lix.systems"
        "https://nix-community.cachix.org"
        "https://hyprland.cachix.org"
      ];
      extra-trusted-public-keys = [
        "cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="
        "krezh.cachix.org-1:0hGx8u/mABpZkzJEBh/UMXyNon5LAXdCRqEeVn5mff8="
        "cache.lix.systems:aBnZUw8zA7H35Cz2RyKFVs3H4PlGTLawyY5KRbvJR8o="
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
        "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
      ];
    };
  };

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";
}
