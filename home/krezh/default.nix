{ inputs, outputs, lib, config, pkgs, hostName, ... }:

{
  imports =
    if (hostName != "thor-wsl") then [
      ../../modules/common
      ./features/cli
      ./features/desktop
      inputs.sops-nix.homeManagerModules.sops

    ] ++ (builtins.attrValues outputs.homeManagerModules) else [
      ../../modules/common
      ./features/cli
      inputs.sops-nix.homeManagerModules.sops

    ] ++ (builtins.attrValues outputs.homeManagerModules);

  nixpkgs = {
    overlays = builtins.attrValues outputs.overlays;
  };

  nix = {
    package = lib.mkDefault pkgs.nix;
    settings = {
      accept-flake-config = true;
      cores = 0;
      max-jobs = "auto";
      experimental-features = [ "nix-command" "flakes" "repl-flake" ];
      warn-dirty = false;
      extra-substituters = [
        "https://krezh.cachix.org"
        "https://nix-community.cachix.org"
        "https://hyprland.cachix.org"
      ];
      extra-trusted-public-keys = [
        "krezh.cachix.org-1:0hGx8u/mABpZkzJEBh/UMXyNon5LAXdCRqEeVn5mff8="
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
        "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
      ];
    };
  };

  xdg.enable = true;

  fonts.fontconfig.enable = true;

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
    };
    packages = with pkgs; [
      inputs.nh.packages.${pkgs.system}.default
      inputs.nixd.packages.${pkgs.system}.nixd
      inputs.nix-fast-build.packages.${pkgs.system}.nix-fast-build
      unstable.fluxcd
      doppler
      wget
      curl
      nodejs
      jq
      ripgrep
      gh
      gcc
      sops
      age
      unstable.go
      go-task
      opentofu
      comma # Install and run programs by sticking a , before them
      bc # Calculator
      bottom # System viewer
      ncdu # TUI disk usage
      eza # Better ls
      ripgrep # Better grep
      fd # Better find
      httpie # Better curl
      diffsitter # Better diff
      jq # JSON pretty printer and manipulator
      timer # To help with my ADHD paralysis
      nil # Nix LSP
      nixfmt # Nix formatter
      nvd # Differ
      nix-output-monitor
      ltex-ls # Spell checking LSP
      dconf
      kubectl-cnpg
      kubectl-node-shell
    ];
  };

  modules.shell.mise = {
    enable = true;
    package = pkgs.unstable.mise;
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
    package = pkgs.unstable.atuin;
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

    bat = {
      enable = true;
      config = {
        paging = "never";
        style = "plain";
        theme = "base16";
      };
    };

    eza = {
      enable = true;
      icons = true;
      enableAliases = true;
    };

    zoxide = {
      enable = true;
      enableFishIntegration = true;
    };

    git = {
      enable = true;
      userName = "Krezh";
      userEmail = "krezh@users.noreply.github.com";
      extraConfig = {
        commit.gpgsign = true;
        pull.rebase = true;
        push.autoSetupRemote = true;
        gpg.format = "ssh";
        format.signoff = true;
        status.submoduleSummary = false;
        tag.forceSignAnnotated = true;
        user.signingkey = config.sops.secrets."ssh/privkey".path;
        init.defaultBranch = "main";
        url."ssh://git@github.com/".pushInsteadOf = "https://github.com/";
      };
    };
  };

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";
}
