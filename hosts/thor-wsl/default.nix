# This is your system's configuration file.
{ inputs, outputs, modulesPath, lib, config, pkgs, ... }:

{
  imports = [
    inputs.nixos-wsl.nixosModules.wsl
    (modulesPath + "/profiles/minimal.nix")

    ../common/global
    ../common/users/krezh
    ./hardware-configuration.nix
  ];

  wsl = {
    enable = true;
    defaultUser = "krezh";
    nativeSystemd = true;
    wslConf.network = {
      hostname = "thor-wsl";
      generateResolvConf = true;
    };
    startMenuLaunchers = false;
    interop.includePath = false;
  };

  boot.isContainer = true;

  # doesn't work on wsl
  services.dbus.apparmor = "disabled";
  services.resolved.enable = false;
  networking.networkmanager.enable = false;
  networking.hostName = "thor-wsl";
  security = {
    sudo.wheelNeedsPassword = true;
    apparmor.enable = false;
    audit.enable = false;
    auditd.enable = false;
  };

  programs.fish.enable = true;

  # https://nixos.wiki/wiki/FAQ/When_do_I_update_stateVersion
  system.stateVersion = "23.11";
}
