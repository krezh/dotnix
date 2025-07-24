{
  inputs,
  lib,
  hostname,
  ...
}:
{
  imports =
    [
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

  nixosModules.desktop.fonts.enable = true;

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

  programs.hyprland = {
    enable = true;
  };

  security.rtkit.enable = true;

  networking.hostName = "${hostname}";
}
