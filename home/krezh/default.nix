{ inputs, outputs, lib, config, pkgs, hostName, ... }:

{
  imports = if (hostName != "thor-wsl") then
    [
      ../../modules/common
      ./features/cli
      ./features/desktop
      inputs.sops-nix.homeManagerModules.sops

    ] ++ (builtins.attrValues outputs.homeManagerModules)
  else
    [
      ../../modules/common
      ./features/cli
      inputs.sops-nix.homeManagerModules.sops

    ] ++ (builtins.attrValues outputs.homeManagerModules);

  # nixpkgs = { overlays = builtins.attrValues outputs.overlays; };

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
      "atuin/key" = { path = "${config.xdg.configHome}/atuin/key"; };
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
      inputs.nix-fast-build.packages.${pkgs.system}.nix-fast-build
      inputs.talosctl.packages.${pkgs.system}.talosctl
      ansible
      cachix
      fluxcd
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
      go
      dyff
      go-task
      opentofu
      niv
      kubectl
      kubeswitch
      cargo
      comma
      bc
      bottom
      ncdu
      ripgrep
      fd
      httpie
      diffsitter
      jq
      timer
      nil
      nixfmt
      nvd
      nix-output-monitor
      ltex-ls
      dconf
      kubectl-cnpg
      kubectl-node-shell
      ntfy-sh
      procs
      hwatch
      envsubst
      gopls
      gotools
      stern
    ];
  };

  modules.shell.krew = {
    enable = true;
    package = pkgs.krew;
  };

  modules.shell.kubectx = { enable = true; };

  modules.shell.mise = {
    enable = true;
    package = pkgs.mise;
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
