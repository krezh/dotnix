{
  inputs,
  outputs,
  lib,
  config,
  pkgs,
  hostName,
  ...
}:
let
  isDesktop = hostName != "thor-wsl";
in
{
  imports = [
    ./features/cli
    inputs.sops-nix.homeManagerModules.sops
    inputs.catppuccin.homeManagerModules.catppuccin
    inputs.nix-index.hmModules.nix-index
  ] ++ (if isDesktop then [ ./features/desktop ] else [ ]) ++ (outputs.commonModules);

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
      "attic/netrc" = { };
    };
  };

  home = {
    username = lib.mkDefault "krezh";
    homeDirectory = lib.mkDefault "/home/${config.home.username}";
    stateVersion = lib.mkDefault "24.05";
    sessionPath = [
      "$HOME/.local/bin"
      "$GOPATH/bin"
      "$CARGO_HOME/bin"
    ];
    sessionVariables = {
      FLAKE = "$HOME/nix-config";
      GOPATH = "${config.xdg.dataHome}/go";
      CARGO_HOME = "${config.xdg.dataHome}/cargo";
      SOPS_AGE_KEY_FILE = "${config.sops.age.keyFile}";
    };
    packages = with pkgs; [
      doppler
      wget
      curl
      ripgrep
      gh
      sops
      age
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
      nitch
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

      # JSON
      jq
      jc
      jnv

      # Nix      
      cachix
      nixfmt-rfc-style
      nvd
      nix-output-monitor
      niv
      comma

      # Kubernetes
      talosctl
      kubectl
      kubeswitch
      kubectl-cnpg
      kubectl-node-shell
      kubectl-klock
      kustomize
      fluxcd
      stern
      helmfile
      kind
    ];
  };

  modules.shell.krew.enable = true;
  modules.shell.kubectx.enable = true;
  modules.shell.aria2.enable = true;

  programs = {
    home-manager.enable = true;
    neomutt.enable = true;
    yazi.enable = true;
    fzf.enable = true;
    zoxide = {
      enable = true;
      enableFishIntegration = true;
    };
    direnv = {
      enable = true;
      nix-direnv.enable = true;
      silent = true;
    };
  };

  nix = {
    # package = lib.mkDefault pkgs.nix;
    settings = {
      accept-flake-config = true;
      cores = 0;
      max-jobs = "auto";
      experimental-features = [
        "nix-command"
        "flakes"
        "repl-flake"
      ];
      trusted-users = [
        "root"
        "@wheel"
      ];
      builders-use-substitutes = true;
      warn-dirty = false;
      netrc-file = config.sops.secrets."attic/netrc".path;
      extra-substituters = [
        #"https://krezh.cachix.org"
        "https://nix-cache.plexuz.xyz/krezh"
        "https://nix-community.cachix.org"
        "https://hyprland.cachix.org"
        "https://anyrun.cachix.org"
        "https://walker-git.cachix.org"
      ];
      extra-trusted-public-keys = [
        #"krezh.cachix.org-1:0hGx8u/mABpZkzJEBh/UMXyNon5LAXdCRqEeVn5mff8="
        "krezh:pqkm/pHp8LD52mFQdGjZR1Xo7RvaG3KdBK4r4FvxIlA="
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
        "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
        "anyrun.cachix.org-1:pqBobmOjI7nKlsUMV25u9QHa9btJK65/C8vnO3p346s="
        "walker-git.cachix.org-1:vmC0ocfPWh0S/vRAQGtChuiZBTAe4wiKDeyyXM0/7pM="
      ];
    };
  };

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";
}
