{
  inputs,
  pkgs,
  lib,
  hostname,
  ...
}:
{
  imports =
    [
      #inputs.hyprland.nixosModules.default
      inputs.jovian.nixosModules.default
    ]
    ++ (lib.scanPath.toList { path = ../common/users; })
    ++ (lib.scanPath.toList { path = ../common/global; });

  jovian = {
    devices.steamdeck = {
      enable = true;
      autoUpdate = true;
    };

    steam = {
      enable = true;
      autoStart = true;
      user = "krezh";
      desktopSession = "hyprland";
    };
  };

  services = {
    fstrim.enable = true;

    libinput = {
      enable = true;
      mouse = {
        accelProfile = "flat";
      };
      touchpad = {
        accelProfile = "flat";
      };
    };

    pipewire = {
      enable = true;
      alsa = {
        enable = true;
        support32Bit = true;
      };
      pulse = {
        enable = true;
      };
      jack = {
        enable = true;
      };
      wireplumber = {
        enable = true;
      };
      audio = {
        enable = true;
      };
    };
  };

  networking.networkmanager.enable = true;

  fonts = {
    fontDir.enable = true;
    enableDefaultPackages = true;
    packages = with pkgs; [
      noto-fonts
      noto-fonts-cjk
      noto-fonts-emoji
      liberation_ttf
      fira-code
      fira-code-symbols
      mplus-outline-fonts.githubRelease
      dina-font
      proggyfonts
      (nerdfonts.override {
        fonts = [
          "CascadiaCode"
          "DroidSansMono"
        ];
      })
    ];
  };

  programs.hyprland = {
    enable = true;
    #package = inputs.hyprland.packages.${pkgs.system}.hyprland;
    #portalPackage = inputs.xdg-portal-hyprland.packages.${pkgs.system}.default;
  };

  security.rtkit.enable = true;

  networking.hostName = "${hostname}";
}
