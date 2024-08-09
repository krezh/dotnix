{
  inputs,
  modulesPath,
  lib,
  pkgs,
  ...
}:
{
  imports =
    [
      inputs.nixos-wsl.nixosModules.wsl
      inputs.vscode-server.nixosModules.default
      (modulesPath + "/profiles/minimal.nix")

    ]
    ++ (lib.scanPath.toList { path = ../common/users; })
    ++ (lib.scanPath.toList { path = ../common/global; });

  wsl = {
    enable = true;
    defaultUser = "krezh";
    nativeSystemd = true;
    wslConf.network = {
      hostname = "thor-wsl";
      generateResolvConf = true;
    };
    startMenuLaunchers = false;
    interop.includePath = true;
    useWindowsDriver = true;
  };

  programs.nix-ld = {
    enable = true;
    package = pkgs.nix-ld-rs;
  };

  environment.sessionVariables = {
    PODMAN_IGNORE_CGROUPSV1_WARNING = "true";
  };

  # services.vscode-server.enable = true;
  # services.vscode-server.enableFHS = true;

  boot.isContainer = true;

  services = {
    dbus.apparmor = "disabled";
    resolved.enable = false;
  };

  networking = {
    networkmanager.enable = false;
    hostName = "thor-wsl";
  };

  security = {
    apparmor.enable = false;
    audit.enable = false;
    auditd.enable = false;
  };
}
