{
  flake.modules.nixos.thor = {
    nixosModules.wireplumber = {
      enable = true;
      audioSwitching = {
        enable = true;
        primary = "A50";
        secondary = "Argon Speakers";
      };
      deviceSettings = {
        "usb-Generic_USB_Audio-00" = {
          deviceProps = {
            "device.profile" = "pro-audio";
            "device.restore-profile" = false;
          };
        };
      };
      hideNodes = [
        "alsa_output.usb-Generic_USB_Audio-00.pro-output-0"
        "alsa_output.usb-Generic_USB_Audio-00.pro-output-1"
        "alsa_output.usb-Generic_USB_Audio-00.pro-output-3"
        "alsa_input.usb-Generic_USB_Audio-00.pro-input-0"
        "alsa_input.usb-Generic_USB_Audio-00.pro-input-1"
        "alsa_input.usb-Generic_USB_Audio-00.pro-input-2"
      ];
      renameModules = [
        {
          nodeName = "alsa_output.usb-Generic_USB_Audio-00.pro-output-2";
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
