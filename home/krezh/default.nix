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
  imports =
    [
      ../../modules/common
      ./features/cli
      inputs.sops-nix.homeManagerModules.sops
      inputs.catppuccin.homeManagerModules.catppuccin
    ]
    ++ (if isDesktop then [ ./features/desktop ] else [ ])
    ++ (builtins.attrValues outputs.homeManagerModules);

  nixpkgs = {
    overlays = builtins.attrValues outputs.overlays;
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

  xdg.enable = true;

  catppuccin = {
    flavor = "mocha";
    enable = true;
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
    stateVersion = lib.mkDefault "23.11";
    sessionPath = [ "$HOME/.local/bin" ];
    sessionVariables = {
      FLAKE = "$HOME/nix-config";
      DEFAULT_BROWSER = "${pkgs.firefox}/bin/firefox";
    };
    packages = with pkgs; [
      inputs.nh.packages.${pkgs.system}.default
      inputs.nixd.packages.${pkgs.system}.nixd
      inputs.talosctl.packages.${pkgs.system}.talosctl
      cachix
      fluxcd
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
      niv
      comma
      bottom
      ncdu
      fd
      httpie
      diffsitter
      jq
      timer
      nixfmt-rfc-style
      nvd
      nix-output-monitor
      
      # Kubernetes
      kubectl
      kubeswitch
      kubectl-cnpg
      kubectl-node-shell
      kubectl-klock

      ntfy-sh
      procs
      hwatch
      envsubst
      gopls
      stern
    ];
  };

  modules.shell.krew = {
    enable = true;
  };

  modules.shell.kubectx = {
    enable = true;
  };

  modules.shell.mise = {
    enable = true;
    config = {
      python_venv_auto_create = true;
      status = {
        missing_tools = "always";
        show_env = false;
        show_tools = false;
      };
    };
  };

  modules.shell.atuin = {
    enable = true;
    package = pkgs.atuin;
    sync_address = "https://sh.talos.plexuz.xyz";
    config = {
      key_path = config.sops.secrets."atuin/key".path;
      style = "compact";
      workspaces = true;
    };
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

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";
}
