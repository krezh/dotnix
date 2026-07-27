{
  flake.modules.nixos.thor = {
    nixosModules.wireplumber = {
      enable = true;
      audioSwitching = {
        enable = true;
        primary = "A50 Game";
        secondary = "Argon Speakers";
      };
      deviceSettings = {
        "alsa_card.usb-Generic_USB_Audio-00" = {
          deviceProps = {
            "device.profile" = "HiFi 5 + 1";
            "device.restore-profile" = false;
          };
        };
        "alsa_card.usb-Logitech_A50-00" = {
          deviceProps = {
            "device.profile" = "pro-audio";
            "device.restore-profile" = false;
          };
        };
      };
      hideNodes = [
        "alsa_output.usb-Generic_USB_Audio-00.HiFi_5_1__Speaker__sink"
        "alsa_output.usb-Generic_USB_Audio-00.HiFi_5_1__Headphones__sink"
        "alsa_input.usb-Generic_USB_Audio-00.HiFi_5_1__Mic__source"
        "alsa_input.usb-Generic_USB_Audio-00.HiFi_5_1__Line__source"
        "alsa_input.usb-Logitech_A50-00.pro-input-1"
      ];
      renameModules = [
        {
          nodeName = "alsa_output.usb-Generic_USB_Audio-00.HiFi_5_1__SPDIF__sink";
          description = "Argon Speakers";
          nick = "Argon Speakers";
        }
        {
          nodeName = "alsa_output.usb-Logitech_A50-00.pro-output-1";
          description = "A50 Game";
          nick = "A50 Game";
        }
        {
          nodeName = "alsa_output.usb-Logitech_A50-00.pro-output-0";
          description = "A50 Chat";
          nick = "A50 Chat";
        }

        {
          nodeName = "alsa_input.usb-Logitech_A50-00.pro-input-0";
          description = "A50 Chat";
          nick = "A50";
        }
      ];
    };
  };
}
