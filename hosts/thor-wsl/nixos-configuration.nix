{
  inputs,
  modulesPath,
  lib,
  ...
}:
{
  imports =
    [
      inputs.nixos-wsl.nixosModules.wsl
      inputs.vscode-server.nixosModules.default
      (modulesPath + "/profiles/minimal.nix")

    ]
    ++ (lib.listNixFiles { path = ../common/users; })
    ++ (lib.listNixFiles { path = ../common/global; });

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

  environment.sessionVariables = {
    PODMAN_IGNORE_CGROUPSV1_WARNING = "true";
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
