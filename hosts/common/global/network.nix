{ config, ... }:
{
  networking.networkmanager.ensureProfiles.environmentFiles = [
    config.sops.templates."networkManager.env".path
  ];
  networking.networkmanager.ensureProfiles.profiles = {
    Plexuz = {
      connection = {
        id = "Plexuz";
        type = "wifi";
        autoconnect = true;
        zone = "trusted";
        interface-name = "wlp45s0";
      };
      wifi = {
        ssid = "Plexuz";
        mode = "infrastructure";
      };
      wifi-security = {
        auth-alg = "open";
        key-mgmt = "wpa-psk";
        psk = "$Plexuz";
      };
      ipv4 = {
        method = "auto";
      };
      ipv6 = {
        method = "auto";
        addr-gen-mode = "default";
      };
      proxy = { };
    };
    Flyn = {
      connection = {
        id = "Flyn";
        type = "wifi";
        autoconnect = true;
        zone = "trusted";
        interface-name = "wlp45s0";
      };
      wifi = {
        ssid = "Flyn151";
        mode = "infrastructure";
      };
      wifi-security = {
        auth-alg = "open";
        key-mgmt = "wpa-psk";
        psk = "$Flyn";
      };
      ipv4 = {
        method = "auto";
      };
      ipv6 = {
        method = "auto";
        addr-gen-mode = "default";
      };
      proxy = { };
    };
  };
}
