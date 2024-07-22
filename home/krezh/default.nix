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
      SOPS_AGE_KEY_FILE = "${config.xdg.configHome}/sops/age/keys.txt";
    };
    packages = with pkgs; [
      doppler
      wget
      curl
      jq
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
      jq
      timer
      bottom
      ffmpeg
      yt-dlp
      nitch
      earthly

      # Nix
      inputs.nh.packages.${pkgs.system}.default
      inputs.nixd.packages.${pkgs.system}.nixd
      inputs.nix-update.packages.${pkgs.system}.nix-update
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

      ntfy-sh
      procs
      hwatch
      envsubst
      gopls
    ];
  };

  modules.shell.krew = {
    enable = true;
  };

  modules.shell.kubectx = {
    enable = true;
  };

  programs = {
    home-manager.enable = true;
    neomutt.enable = true;
    yazi.enable = true;
    fzf.enable = true;

    zoxide = {
      enable = true;
      enableFishIntegration = true;
    };
  };

  nix = {
    package = lib.mkDefault pkgs.nix;
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
      extra-substituters = [
        "https://krezh.cachix.org"
        "https://nix-community.cachix.org"
        "https://hyprland.cachix.org"
        "https://anyrun.cachix.org"
      ];
      extra-trusted-public-keys = [
        "krezh.cachix.org-1:0hGx8u/mABpZkzJEBh/UMXyNon5LAXdCRqEeVn5mff8="
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
        "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
        "anyrun.cachix.org-1:pqBobmOjI7nKlsUMV25u9QHa9btJK65/C8vnO3p346s="
      ];
    };
  };

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";
}
