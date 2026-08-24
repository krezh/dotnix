{
  description = "Krezh's NixOS Flake";
  nixConfig = {
    extra-trusted-substituters = [
      "https://xilo.plexuz.xyz/c/admin/krezh"
      "https://nix-community.cachix.org"
    ];
    extra-trusted-public-keys = [
      "krezh:orJlBHtC8lGYpXoH6ORLMpBR7zrgfgGIgm1/xT8Lbvs="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/e5bdc4a41d4c072fe1e3787eaa0320a384741d44";
    hardware.url = "github:nixos/nixos-hardware/ff17823245ab9ff7bcae6acf950bd89cba82c38c";
    flake-parts = {
      url = "github:hercules-ci/flake-parts/427bf4bd9435fdf21321c8cc628c24efc14c0f7a";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    nix-cachyos-kernel = {
      url = "github:xddxdd/nix-cachyos-kernel/dbae8dc454556968710401643aa2c6867d166440";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-parts.follows = "flake-parts";
      };
    };

    lanzaboote = {
      url = "github:nix-community/lanzaboote/v1.1.0";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    home-manager = {
      url = "github:nix-community/home-manager/353742587cbaf079b3caee743115d037bc51fea6";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    devshell = {
      url = "github:numtide/devshell/255a2b1725a20d060f566e4755dbf571bbbb5f76";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt-nix = {
      url = "github:numtide/treefmt-nix/27b3b12a8e6375f28ebe122f07d230ca5459bbfa";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-eval-jobs = {
      url = "github:nix-community/nix-eval-jobs/v2.34.3";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-parts.follows = "flake-parts";
        treefmt-nix.follows = "treefmt-nix";
      };
    };

    disko = {
      url = "github:nix-community/disko/ff8702b4de27f72b4c78573dfb89ec74e36abdf1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    impermanence = {
      url = "github:nix-community/impermanence/7b1d382faf603b6d264f58627330f9faa5cba149";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    catppuccin = {
      url = "github:catppuccin/nix/35d78c213b65e38789bcb359aae2380fcb4dc3e8";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-index = {
      url = "github:nix-community/nix-index-database/c7962dc97b45129df8d751bedaf37beb5a17706e";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    sops-nix = {
      url = "github:Mic92/sops-nix/a8627b21b9107c5711c96b84f32a9a4b3d45295f";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nvf = {
      url = "github:notashelf/nvf/59b0dc327b5ce7a6b510c50456a35891ddd144cc";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    nixos-wsl = {
      url = "github:nix-community/nixos-wsl/eaeb18da90024448a60eb1ec7132eafa4003404e";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    zen-browser = {
      url = "github:0xc000022070/zen-browser-flake/fd7d5b08ac2e4da35cf908cbbf762a9276f00ddd";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        home-manager.follows = "home-manager";
      };
    };

    zen-browser-catppuccin = {
      url = "github:catppuccin/zen-browser/dbfa3f6b29ef46b57375a3745f20bb7a50702727";
      flake = false;
    };

    code-comments = {
      url = "github:pash7ka/code-comments/f6e1493b5f69c34f0017455a94d5d4e730461b06";
      flake = false;
    };

    helium = {
      url = "github:cjavad/nixpille-helium/d05088ecc155780e95e63dd0fd03c7a9df331817";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    steam-config-nix = {
      url = "github:different-name/steam-config-nix/30dc17418e7aff0b78ee14ef6c451b5f3422e792";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    jovian = {
      url = "github:Jovian-Experiments/Jovian-NixOS/a6043f17992543f9157c594129272e7621c45ea5";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    elephant = {
      url = "github:abenz1267/elephant/23f37238367355cf46843015ad5e94706200176a";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    walker = {
      url = "github:abenz1267/walker/42b3ed88abf50bc52638fb2835b7f17e3ea3ac4c";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        elephant.follows = "elephant";
      };
    };

    kauth = {
      url = "github:krezh/kauth/0.2.28";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        go-overlay.follows = "go-overlay";
      };
    };

    go-overlay = {
      url = "github:purpleclay/go-overlay/faebe5d8158e75e90750783f43bc07f72d523544";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    noctalia = {
      url = "github:noctalia-dev/noctalia-shell/b5c1bfaace0c3b1b36f227970e389f9d20295937";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane.url = "github:ipetkov/crane/692f7e9ef2ece8125b466f66f2af532b3edaed0d";

    rust-overlay = {
      url = "github:oxalica/rust-overlay/b32685dd7c5a965aa8273adb7ddaf7f5b40d0faa";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    llm-agents-nix = {
      url = "github:numtide/llm-agents.nix/50e05f7a7cc039d39d53bc77c132d9457f990375";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    nix4vscode = {
      url = "github:nix-community/nix4vscode/777db2c5664fc429f739908bcad03f261f8e7045";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    silentSDDM = {
      url = "github:uiriansan/SilentSDDM/0dc4386ef570659537b480f4b226b3b6f2699948";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    spicetify-nix = {
      url = "github:Gerg-L/spicetify-nix/0f478ff79b82abb785160cd4531293f61d21be86";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    fast-nix-gc = {
      url = "github:Mic92/fast-nix-gc/254a2ba0a4f1570b3880bc10bb6166afe1e25936";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    comin = {
      url = "github:nlewo/comin/e72d8cc7ad188dbb109994cba9babf026bacf6ab";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
      };
    };

    hyprland-scroll-overview = {
      url = "github:yayuuu/hyprland-scroll-overview/f9248ab6bee770e9d68813b48cc6ca12b3271254";
      flake = false;
    };

    herdr = {
      url = "github:ogulcancelik/herdr/2d24950ad9a02096921bc764e2e3dcd8900c3366";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    undo = {
      url = "github:edaywalid/undo/513536e2a4218b9f7ad22f16ceb88ab6c2e89292";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nixcord = {
      url = "github:4evy/nixcord/befe06adc4df1d8ea4ebc3fabb92a22e0cecccf1";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        nixpkgs-nixcord.follows = "nixpkgs";
        flake-parts.follows = "flake-parts";
      };
    };

    xilo = {
      url = "github:stubbedev/xilo/857923e34af55ad0c92b86233b55d9172b5868a2";
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
