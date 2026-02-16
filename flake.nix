{
  description = "Krezh's NixOS Flake";
  nixConfig = {
    extra-trusted-substituters = [
      "https://nix-cache.talos.plexuz.xyz/krezh"
      "https://krezh.cachix.org"
      "https://cache.garnix.io"
      "https://nix-community.cachix.org"
      "https://niri.cachix.org"
    ];
    extra-trusted-public-keys = [
      "krezh:+uoEkr8YWCGOBbQgV3H6VPL9Li4/j2rfgC2GxEF3fY8="
      "krezh.cachix.org-1:0hGx8u/mABpZkzJEBh/UMXyNon5LAXdCRqEeVn5mff8="
      "cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "niri.cachix.org-1:Wv0OmO7PsuocRKzfDoJ3mulSl7Z6oezYhGhR+3W2964="
    ];
  };

  inputs = {
    nixpkgs.url = "git+https://github.com/NixOS/nixpkgs?ref=nixos-unstable&shallow=1";
    hardware.url = "git+https://github.com/nixos/nixos-hardware?shallow=1";
    nix-cachyos-kernel.url = "git+https://github.com/xddxdd/nix-cachyos-kernel?shallow=1";

    home-manager = {
      url = "git+https://github.com/nix-community/home-manager?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-parts.url = "git+https://github.com/hercules-ci/flake-parts?shallow=1";

    devshell = {
      url = "git+https://github.com/numtide/devshell?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt-nix = {
      url = "git+https://github.com/numtide/treefmt-nix?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    disko = {
      url = "git+https://github.com/nix-community/disko?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    catppuccin = {
      url = "git+https://github.com/catppuccin/nix?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-index = {
      url = "git+https://github.com/nix-community/nix-index-database?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    sops-nix = {
      url = "git+https://github.com/Mic92/sops-nix?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nvf = {
      url = "git+https://github.com/notashelf/nvf?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixos-wsl = {
      url = "git+https://github.com/nix-community/nixos-wsl?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    zen-browser = {
      url = "git+https://github.com/0xc000022070/zen-browser-flake?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.home-manager.follows = "home-manager";
    };

    zen-browser-catppuccin = {
      url = "git+https://github.com/catppuccin/zen-browser?shallow=1";
      flake = false;
    };

    nix-gaming.url = "git+https://github.com/fufexan/nix-gaming?shallow=1";

    jovian = {
      url = "git+https://github.com/Jovian-Experiments/Jovian-NixOS?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    elephant = {
      url = "git+https://github.com/abenz1267/elephant?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    walker = {
      url = "git+https://github.com/abenz1267/walker?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.elephant.follows = "elephant";
    };

    kauth = {
      url = "git+https://github.com/krezh/kauth?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    go-overlay = {
      url = "git+https://github.com/purpleclay/go-overlay?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    noctalia = {
      url = "git+https://github.com/noctalia-dev/noctalia-shell?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-ai-tools = {
      url = "git+https://github.com/numtide/nix-ai-tools?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.treefmt-nix.follows = "treefmt-nix";
    };

    niri = {
      url = "git+https://github.com/sodiboo/niri-flake?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixos-generators = {
      url = "git+https://github.com/nix-community/nixos-generators?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix4vscode.url = "git+https://github.com/nix-community/nix4vscode?shallow=1";
    nix4vscode.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    let
      lib = import ./lib { inherit inputs; };
    in
    flake-parts.lib.mkFlake
      {
        inherit inputs;
        specialArgs = { inherit lib; };
      }
      {
        imports = lib.scanPath.toImports ./modules;
      };
}
