{ inputs, outputs, lib, config, pkgs, ... }:

{
  imports = [
    ../../modules/common
    ./features/cli
    inputs.impermanence.nixosModules.home-manager.impermanence
    inputs.sops-nix.homeManagerModules.sops
  ] ++ (builtins.attrValues outputs.homeManagerModules);

  nixpkgs = {
    overlays = builtins.attrValues outputs.overlays;
    config = {
      allowUnfree = true;
      allowUnfreePredicate = (_: true);
    };
  };

  nix = {
    package = lib.mkDefault pkgs.nix;
    settings = {
      experimental-features = [ "nix-command" "flakes" "repl-flake" ];
      warn-dirty = false;
    };
  };

  home = {
    username = lib.mkDefault "krezh";
    homeDirectory = lib.mkDefault "/home/${config.home.username}";
    stateVersion = lib.mkDefault "23.11";
    sessionPath = [ "$HOME/.local/bin" ];
    sessionVariables = { FLAKE = "$HOME/nix-config"; };

    persistence = {
      "/mnt/wsl/home/${config.home.username}" = {
        directories = [
          "Documents"
          "Downloads"
          "Pictures"
          "Videos"
          ".local/bin"
          ".local/share/nix" # trusted settings and repl history
        ];
        allowOther = true;
      };
    };
  };

  sops = {
    age.keyFile = "/home/${config.home.username}/.config/sops/age/keys.txt";
    defaultSopsFile = ./secrets.sops.yaml;
    secrets = {
      hello = { path = "/home/${config.home.username}/nix-config/hello"; };
    };
  };

  # Add stuff for your user as you see fit:
  home.packages = with pkgs; [
    inputs.nh.packages.${pkgs.system}.default
    inputs.nixd.packages.${pkgs.system}.nixd
    inputs.nix-fast-build.packages.${pkgs.system}.nix-fast-build
    wget
    curl
    nodejs
    jq
    ripgrep
    gh
    gcc
    sops
    age
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
  ];

  modules.shell.mise = {
    enable = true;
    config = {
      python_venv_auto_create = true;
      status = {
        missing_tools = "if_other_versions_installed";
        show_env = false;
        show_tools = true;
      };
    };
  };

  programs = {
    home-manager.enable = true;

    neomutt = { enable = true; };

    neovim = {
      enable = true;
      viAlias = true;
      vimAlias = true;
      defaultEditor = true;
      extraConfig = ''
        set number relativenumber
      '';
    };

    yazi = { enable = true; };

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

    fzf.enable = true;

    git = {
      enable = true;
      userName = "Krezh";
      userEmail = "krezh@users.noreply.github.com";
    };
  };

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";
}
