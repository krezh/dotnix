{ inputs, modulesPath, ... }:
{
  imports = [
    inputs.nixos-wsl.nixosModules.wsl
    inputs.vscode-server.nixosModules.default
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
    interop.includePath = true;
    useWindowsDriver = true;
  };

  services.vscode-server.enable = true;
  services.vscode-server.enableFHS = true;

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
