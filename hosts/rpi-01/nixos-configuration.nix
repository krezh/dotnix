{
  pkgs,
  lib,
  hostname,
  ...
}:
{
  imports =
    [ ]
    ++ (lib.scanPath.toList { path = ../common/users; })
    ++ (lib.scanPath.toList { path = ../common/global; });

  boot = {
    kernelPackages = pkgs.linuxPackages_latest;
  };

  services = {
    fstrim.enable = true;
  };

  networking.networkmanager.enable = true;

  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ ];
    allowedUDPPorts = [ ];
  };

  environment = {
    sessionVariables = { };
    systemPackages = [ ];
  };
  networking.hostName = "${hostname}";
}
