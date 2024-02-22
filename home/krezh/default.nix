{ inputs, outputs, lib, config, pkgs, ... }:

{
  imports = [
    ../../modules/common
    ./features/cli
    ./features/desktop/terminal
    inputs.sops-nix.homeManagerModules.sops
    inputs.nixvim.homeManagerModules.nixvim
    inputs.hyprlock.homeManagerModules.hyprlock
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

  gtk = {
    enable = true;
    theme = {
      name = "Catppuccin-Mocha-Compact-Maroon-Dark";
      package = pkgs.catppuccin-gtk.override {
        accents = [ "maroon" ];
        size = "compact";
        tweaks = [ "rimless" "black" ];
        variant = "mocha";
      };
    };
  };

  wayland.windowManager.hyprland = {
    enable = true;
    package = inputs.hyprland.packages.${pkgs.system}.hyprland;
    systemd.enable = true;
    xwayland.enable = true;
    extraConfig = ''
      ${builtins.readFile ./hypr.conf}
    '';
    settings = {
      "$mod" = "SUPER";
      input = {
        kb_layout = "se";
        follow_mouse = 1;
        accel_profile = "flat";
        sensitivity = 0;
        touchpad = {
          natural_scroll = "false";
        };
      };
      monitor = [
        ",preferred,auto,1"
      ];
      bind = [
        "$mod,        RETURN, exec, wezterm"
        "$mod, 	      L,      exec, hyprlock"
        "$mod,        Q,      killactive"
        "$mod,        V,      togglefloating"
        "$mod SHIFT,  LEFT,    movewindow, l"
        "$mod SHIFT,  RIGHT,   movewindow, r"
        "$mod SHIFT,  UP,      movewindow, u"
        "$mod SHIFT,  RIGHT,   movewindow, d"
      ];
    };
  };

  sops = {
    age.keyFile = "/home/${config.home.username}/.config/sops/age/keys.txt";
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
    sessionPath = [ "$HOME/.local/bin" ];
    sessionVariables = { FLAKE = "$HOME/nix-config"; };
    pointerCursor = {
      gtk.enable = true;
      x11.enable = true;
      package = pkgs.bibata-cursors;
      name = "Bibata-Modern-Amber";
      size = 28;
    };
    packages = with pkgs; [
      inputs.nh.packages.${pkgs.system}.default
      inputs.nixd.packages.${pkgs.system}.nixd
      inputs.nix-fast-build.packages.${pkgs.system}.nix-fast-build
      unstable.fluxcd
      firefox
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
    hyprlock.enable = true;

    nixvim = {
      enable = true;
      viAlias = true;
      vimAlias = true;
      defaultEditor = true;
      plugins = {
        lightline.enable = true;
        treesitter.enable = true;
        telescope.enable = true;
        oil.enable = true;
        lsp = {
          enable = true;
          servers = {
            lua-ls.enable = true;
            gopls.enable = true;
          };
        };
        nvim-cmp = {
          enable = true;
          autoEnableSources = true;
          sources = [
            { name = "nvim_lsp"; }
            { name = "path"; }
            { name = "buffer"; }
            { name = "emoji"; }
          ];
        };
      };
      colorschemes.catppuccin.enable = true;
      # extraPlugins = with pkgs.vimPlugins; [
      #   vim-nix
      # ];
      options = {
        number = true; # Show line numbers
        relativenumber = true; # Show relative line numbers
        shiftwidth = 2; # Tab width should be 2
      };
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
