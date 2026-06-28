{
  description = "Krezh's NixOS Flake";
  nixConfig = {
    extra-trusted-substituters = [
      # "https://nix-cache.plexuz.xyz/krezh"
      "https://niks.plexuz.xyz"
      "https://nix-community.cachix.org"
    ];
    extra-trusted-public-keys = [
      # "krezh:GBrZyWDPWYTg/9a9Vad/NRQF/1w0Yc1kWXOQXM3d0RQ="
      "niks.plexuz.xyz-1:dBHlH3p4D7VL2bEW3csdOtJ/X3HTWiCUapJfua48DMg="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };

  inputs = {
    nixpkgs.url = "git+https://github.com/NixOS/nixpkgs?ref=nixos-unstable&shallow=1";
    hardware.url = "git+https://github.com/nixos/nixos-hardware?shallow=1";
    flake-parts.url = "git+https://github.com/hercules-ci/flake-parts?shallow=1";
    nix-cachyos-kernel = {
      url = "git+https://github.com/xddxdd/nix-cachyos-kernel?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    lanzaboote = {
      url = "git+https://github.com/nix-community/lanzaboote?ref=refs/tags/v1.1.0&shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    home-manager = {
      url = "git+https://github.com/nix-community/home-manager?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

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

    impermanence = {
      url = "git+https://github.com/nix-community/impermanence?shallow=1";
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

    niks3 = {
      url = "git+https://github.com/Mic92/niks3?shallow=1";
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
      url = "github:catppuccin/zen-browser/dbfa3f6b29ef46b57375a3745f20bb7a50702727";
      flake = false;
    };

    helium = {
      url = "git+https://github.com/cjavad/nixpille-helium?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-gaming = {
      url = "git+https://github.com/fufexan/nix-gaming?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    steam-config-nix = {
      url = "git+https://github.com/different-name/steam-config-nix?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

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

    crane.url = "git+https://github.com/ipetkov/crane?shallow=1";

    rust-overlay = {
      url = "git+https://github.com/oxalica/rust-overlay?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    llm-agents-nix = {
      url = "git+https://github.com/numtide/llm-agents.nix?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.treefmt-nix.follows = "treefmt-nix";
    };

    nix4vscode = {
      url = "git+https://github.com/nix-community/nix4vscode?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    silentSDDM = {
      url = "git+https://github.com/uiriansan/SilentSDDM?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    spicetify-nix = {
      url = "git+https://github.com/Gerg-L/spicetify-nix?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
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
        debug = true;
        imports = lib.scanPath.toImports ./modules;
      };
}
