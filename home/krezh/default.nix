{ inputs, outputs, lib, config, pkgs, ... }:

{
  imports = [
    ../../modules/common
    ./features/cli
    inputs.sops-nix.homeManagerModules.sops
    inputs.nixvim.homeManagerModules.nixvim
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
    };
  };

  xdg.enable = true;

  sops = {
    age.keyFile = "/home/${config.home.username}/.config/sops/age/keys.txt";
    defaultSopsFile = ./secrets.sops.yaml;
    gnupg.sshKeyPaths = [ ];
    secrets = {
      "ssh/privkey" = {
        path = "/home/${config.home.username}/.ssh/id_ed25519";
        mode = "0600";
      };
    };
  };

  home = {
    username = lib.mkDefault "krezh";
    homeDirectory = lib.mkDefault "/home/${config.home.username}";
    stateVersion = lib.mkDefault "23.11";
    sessionPath = [ "$HOME/.local/bin" ];
    sessionVariables = { FLAKE = "$HOME/nix-config"; };
    packages = with pkgs; [
      inputs.nh.packages.${pkgs.system}.default
      inputs.nixd.packages.${pkgs.system}.nixd
      inputs.nix-fast-build.packages.${pkgs.system}.nix-fast-build
      inputs.talhelper.packages.${pkgs.system}.default
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

  programs = {
    home-manager.enable = true;
    neomutt.enable = true;
    yazi.enable = true;
    fzf.enable = true;

    nixvim = {
      enable = true;
      viAlias = true;
      vimAlias = true;
      defaultEditor = true;
      plugins.lightline.enable = true;
      colorschemes.catppuccin.enable = true;
      extraPlugins = with pkgs.vimPlugins; [
        LazyVim
        vim-nix
      ];
      options = {
        number = true; # Show line numbers
        relativenumber = true; # Show relative line numbers
        shiftwidth = 2; # Tab width should be 2
      };
    };

    #neovim = {
    #  enable = true;
    #  viAlias = true;
    #  vimAlias = true;
    #  defaultEditor = true;
    #  extraConfig = ''
    #      set number relativenumber
    #    	set tabstop=2
    #      set expandtab
    #      set shiftwidth=2
    #  '';
    #};

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
        pull.rebase = true;
        push.autoSetupRemote = true;
        gpg.format = "ssh";
        format.signoff = true;
        status.submoduleSummary = false;
        tag.forceSignAnnotated = true;
        user.signingkey = config.sops.secrets."ssh/privkey".path;
        init.defaultBranch = "main";
        #gpg.ssh.allowedSignersFile = "~/.ssh/allowed_signers";
        url."ssh://git@github.com/".pushInsteadOf = "https://github.com/";
      };
    };
  };

  # home.file.".ssh/allowed_signers".text =
  #   "* ${builtins.readFile /home/${config.home.username}/.ssh/id_ed25519.pub}";

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";
}
