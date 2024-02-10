{ inputs, outputs, lib, config, pkgs, ... }:

{
  imports = [
    ../../modules/common
    ./features/cli
    #inputs.impermanence.nixosModules.home-manager.impermanence
    inputs.sops-nix.homeManagerModules.sops
  ]; # ++ (builtins.attrValues outputs.homeManagerModules); #TODO: not sure what it does

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
    };
  };

  xdg.enable = true;

  home = {
    username = lib.mkDefault "krezh";
    homeDirectory = lib.mkDefault "/home/${config.home.username}";
    stateVersion = lib.mkDefault "23.11";
    sessionPath = [ "$HOME/.local/bin" ];
    sessionVariables = { FLAKE = "$HOME/nix-config"; };

    # persistence = {
    #   "/mnt/wsl/home/${config.home.username}" = {
    #     directories = [
    #       "Documents"
    #       "Downloads"
    #       "Pictures"
    #       "Videos"
    #       ".local/bin"
    #       ".local/share/nix" # trusted settings and repl history
    #     ];
    #     allowOther = false;
    #   };
    # };
  };

  sops = {
    age.keyFile = "/home/${config.home.username}/.config/sops/age/keys.txt";
    defaultSopsFile = ./secrets.sops.yaml;
    gnupg.sshKeyPaths = [];
    secrets = {
      "ssh/privkey" = {
        path = "/home/${config.home.username}/.ssh/id_ed25519";
        mode = "0600";
        #owner = config.users.users.krezh.name;
        #group = config.users.users.krezh.group;
      };
    };
  };

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
    go
    go-task
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

  programs = {
    home-manager.enable = true;
    neomutt.enable = true;
    yazi.enable = true;
    fzf.enable = true;

    neovim = {
      enable = true;
      viAlias = true;
      vimAlias = true;
      defaultEditor = true;
      extraConfig = ''
        set number relativenumber
      '';
    };

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
        # Sign all commits using ssh key
        commit.gpgsign = true;
        gpg.format = "ssh";
        user.signingkey = "~/.ssh/id_ed25519.pub";
        gpg.ssh.allowedSignersFile = "~/.ssh/allowed_signers";
      };
    };
  };

  home.file.".ssh/allowed_signers".text =
    "* ${builtins.readFile /home/${config.home.username}/.ssh/id_ed25519.pub}";

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";
}
