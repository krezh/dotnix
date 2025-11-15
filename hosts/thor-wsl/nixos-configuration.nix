{
  inputs,
  modulesPath,
  lib,
  pkgs,
  hostname,
  ...
}:
{
  imports = [
    inputs.nixos-wsl.nixosModules.wsl
    (modulesPath + "/profiles/minimal.nix")
  ];

  wsl = {
    enable = true;
    defaultUser = "krezh";
    wslConf.network = {
      hostname = "${hostname}";
      generateResolvConf = true;
    };
    startMenuLaunchers = false;
    interop.includePath = true;
    useWindowsDriver = true;
    usbip = {
      enable = true;
      # Replace this with the BUSID for your Yubikey
      autoAttach = [ "7-4" ];
    };
  };

  nixosModules.desktop = {
    openssh.enable = true;
  };

  services.udev.enable = lib.mkForce true;

  programs.nix-ld = {
    enable = true;
    package = pkgs.nix-ld;
  };

  environment.sessionVariables = {
    PODMAN_IGNORE_CGROUPSV1_WARNING = "true";
  };

  boot.isContainer = true;

  services = {
    dbus.apparmor = "disabled";
    resolved.enable = false;
  };

  networking = {
    networkmanager.enable = false;
    hostName = "${hostname}";
  };

  security = {
    apparmor.enable = false;
    audit.enable = false;
    auditd.enable = false;
  };
}
