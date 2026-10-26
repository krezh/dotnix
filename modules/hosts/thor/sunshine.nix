{
  flake.modules.nixos.thor =
    { lib, ... }:
    {
      services.sunshine = {
        enable = true;
        autoStart = true;
        capSysAdmin = true;
        openFirewall = true;
      };

      services.avahi = {
        publish = {
          enable = lib.mkForce null;
          userServices = lib.mkForce null;
        };
        settings.publish = {
          disable-publishing = false;
          disable-user-service-publishing = false;
          publish-addresses = true;
        };
      };
    };
}
