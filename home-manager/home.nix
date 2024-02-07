{ inputs, lib, config, pkgs, ... }:
{
  imports = [];

  nixpkgs = {
    # You can add overlays here
    overlays = [];
    # Configure your nixpkgs instance
    config = {
      # Disable if you don't want unfree packages
      allowUnfree = true;
      # Workaround for https://github.com/nix-community/home-manager/issues/2942
      allowUnfreePredicate = _: true;
    };
  };

  home = {
    username = "krezh";
    homeDirectory = "/home/krezh";
  };

  # Add stuff for your user as you see fit:
  home.packages = with pkgs; [
    wget
    curl
    nodejs
    nixd
  ];

  programs.home-manager.enable = true;

  programs.starship = {
    enable = true;
    enableFishIntegration = true;
    settings = pkgs.lib.importTOML ../config/starship.toml;
  };

  programs.neovim = {
    enable = true;
    viAlias = true;
    vimAlias = true;
    defaultEditor = true;
    extraConfig = ''
      set number relativenumber
      '';
  };

  programs.yazi = {
    enable = true;
  };
  
  programs.eza = {
    enable = true;
    icons = true;
    enableAliases = true;
  };

  programs.zoxide.enable = true;

  programs.fzf.enable = true;

  programs.fish = {
    enable = true;
    interactiveShellInit = ''
      set fish_greeting # Disable greeting
    '';
    shellAliases = {
      # other
      df = "df -h";
      du = "du -h";
      ls = "eza";
    };
    plugins = [
      { name = "puffer"; src = pkgs.fishPlugins.puffer.src; }
      { name = "fzf-fish"; src = pkgs.fishPlugins.fzf-fish.src; }
      { name = "autopair"; src = pkgs.fishPlugins.autopair.src; }
      { name = "bass"; src = pkgs.fishPlugins.bass.src; }
      { name = "forgit"; src = pkgs.fishPlugins.forgit.src; }
      { name = "zoxide"; src = pkgs.fetchFromGitHub {
          owner = "kidonng";
          repo = "zoxide.fish";
          rev = "bfd5947bcc7cd01beb23c6a40ca9807c174bba0e";
          sha256 = "Hq9UXB99kmbWKUVFDeJL790P8ek+xZR5LDvS+Qih+N4=";
        };
      }
    ];
  };
  
  programs.git = {
    enable = true;
    userName = "Krezh";
    userEmail = "krezh@users.noreply.github.com";
  };

  # Nicely reload system units when changing configs
  systemd.user.startServices = "sd-switch";

  # https://nixos.wiki/wiki/FAQ/When_do_I_update_stateVersion
  home.stateVersion = "23.11";
}
