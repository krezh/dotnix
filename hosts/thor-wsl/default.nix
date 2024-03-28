# This is your system's configuration file.
{ inputs, modulesPath, pkgs, ... }: {
  imports = [
    inputs.nixos-wsl.nixosModules.wsl
    inputs.nixos-wsl-vscode.nixosModules.default
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

  vscode-remote-workaround.enable = true;

  # solution adapted from: https://github.com/K900/vscode-remote-workaround
  # more information: https://github.com/nix-community/NixOS-WSL/issues/238 and https://github.com/nix-community/NixOS-WSL/issues/294
  # systemd.user = {
  #   paths.vscode-remote-workaround = {
  #     wantedBy = [ "default.target" ];
  #     pathConfig.PathChanged = "%h/.vscode-server/bin";
  #   };
  #   services.vscode-remote-workaround.script = ''
  #     for i in ~/.vscode-server/bin/*; do
  #       echo "Fixing vscode-server in $i..."
  #       ln -sf ${pkgs.nodejs_18}/bin/node $i/node
  #     done
  #   '';
  # };

  boot.isContainer = true;

  # doesn't work on wsl
  services.dbus.apparmor = "disabled";
  services.resolved.enable = false;
  networking.networkmanager.enable = false;
  networking.hostName = "thor-wsl";
  security = {
    apparmor.enable = false;
    audit.enable = false;
    auditd.enable = false;
  };
}
