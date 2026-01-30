{ inputs, ... }:
let
  user = "krezh";
in
{
  flake.modules.nixos.thor-wsl =
    {
      lib,
      pkgs,
      modulesPath,
      ...
    }:
    {
      home-manager.users.${user} = {
        imports = with inputs.self.modules.homeManager; [
          system-base
          ai
        ];
      };
      imports = with inputs.self.modules.nixos; [
        system-base
        inputs.self.modules.nixos.${user}
        ai
        inputs.nixos-wsl.nixosModules.wsl
        (modulesPath + "/profiles/minimal.nix")
      ];

      networking.hostName = "thor-wsl";

      # WSL configuration
      wsl = {
        enable = true;
        defaultUser = user;
        wslConf.network = {
          hostname = "thor-wsl";
          generateResolvConf = true;
        };
        startMenuLaunchers = false;
        interop.includePath = true;
        useWindowsDriver = true;
        usbip = {
          enable = true;
          autoAttach = [ "7-4" ];
        };
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

      networking.networkmanager.enable = false;

      security = {
        apparmor.enable = false;
        audit.enable = false;
        auditd.enable = false;
        sudo.wheelNeedsPassword = false;
      };
    };
}
