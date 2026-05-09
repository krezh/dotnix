{
  flake.modules.nixos.thor = {
    nixosModules.wireplumber = {
      enable = true;
      audioSwitching = {
        enable = true;
        primary = "A50";
        secondary = "Argon Speakers";
      };
      hideNodes = [
        "alsa_output.usb-Generic_USB_Audio-00.HiFi_5_1__Headphones__sink"
        "alsa_output.usb-Generic_USB_Audio-00.HiFi_5_1__Speaker__sink"
        "alsa_input.usb-Generic_USB_Audio-00.HiFi_5_1__Mic2__source"
        "alsa_input.usb-Generic_USB_Audio-00.HiFi_5_1__Mic1__source"
        "alsa_input.usb-Generic_USB_Audio-00.HiFi_5_1__Line1__source"
      ];
      renameModules = [
        {
          nodeName = "alsa_output.usb-Generic_USB_Audio-00.HiFi_5_1__SPDIF__sink";
          description = "Argon Speakers";
          nick = "Argon Speakers";
        }
        {
          nodeName = "alsa_output.usb-Logitech_A50-00.iec958-stereo";
          description = "A50";
          nick = "A50";
        }
        {
          nodeName = "alsa_input.usb-Logitech_A50-00.mono-fallback";
          description = "A50";
          nick = "A50";
        }
      ];

    };
  };
}
