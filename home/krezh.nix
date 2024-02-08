{
  inputs,
  outputs,
  lib,
  config,
  pkgs,
  ...
}: let
  user = "krezh";
in {
  imports = [
    ../modules/common
    inputs.sops-nix.homeManagerModules.sops
  ];

  nixpkgs = {
    # You can add overlays here
    overlays = [
      # Add overlays your own flake exports (from overlays and pkgs dir):
      outputs.overlays.additions
      outputs.overlays.modifications
      outputs.overlays.unstable-packages
    ];
    # Configure your nixpkgs instance
    config = {
      # Disable if you don't want unfree packages
      allowUnfree = true;
      # Workaround for https://github.com/nix-community/home-manager/issues/2942
      allowUnfreePredicate = _: true;
    };
  };

  home = {
    username = "${user}";
    homeDirectory = "/home/${user}";
  };

  sops = {
    age.keyFile = "/home/${user}/.config/sops/age/keys"; # must have no password!
    # It's also possible to use a ssh key, but only when it has no password:
    #age.sshKeyPaths = [ "/home/user/path-to-ssh-key" ];
    defaultSopsFile = ../.sops.yaml;
  };
  
  # Add stuff for your user as you see fit:
  home.packages = with pkgs; [
    wget
    curl
    nodejs
    jq
    ripgrep
    gh
    gcc
    sops
    age
    inputs.nixd.packages.${pkgs.system}.nixd
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

    starship = {
      enable = true;
      enableFishIntegration = true;
      settings = pkgs.lib.importTOML ../config/starship.toml;
    };

    neovim = {
      enable = true;
      viAlias = true;
      vimAlias = true;
      defaultEditor = true;
      extraConfig = ''
        set number relativenumber
      '';
    };

    yazi = {
      enable = true;
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

    fzf.enable = true;

    fish = {
      enable = true;
      interactiveShellInit = ''
        set fish_greeting # Disable greeting
      '';
      shellAliases = {
        df = "df -h";
        du = "du -h";
        k = "kubectl";
      };
      plugins = [
        {
          name = "puffer";
          src = pkgs.fishPlugins.puffer.src;
        }
        {
          name = "fzf-fish";
          src = pkgs.fishPlugins.fzf-fish.src;
        }
        {
          name = "autopair";
          src = pkgs.fishPlugins.autopair.src;
        }
        {
          name = "bass";
          src = pkgs.fishPlugins.bass.src;
        }
        {
          name = "forgit";
          src = pkgs.fishPlugins.forgit.src;
        }
        {
          name = "zoxide";
          src = pkgs.fetchFromGitHub {
            owner = "kidonng";
            repo = "zoxide.fish";
            rev = "bfd5947bcc7cd01beb23c6a40ca9807c174bba0e";
            sha256 = "Hq9UXB99kmbWKUVFDeJL790P8ek+xZR5LDvS+Qih+N4=";
          };
        }
      ];
    };

    git = {
      enable = true;
      userName = "Krezh";
      userEmail = "krezh@users.noreply.github.com";
    };
  };

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";

  # https://nixos.wiki/wiki/FAQ/When_do_I_update_stateVersion
  home.stateVersion = "23.11";
}
