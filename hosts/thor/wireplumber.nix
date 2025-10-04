{

  nixosModules.desktop.wireplumber = {
    enable = true;
    audioSwitching = {
      enable = true;
      primary = "A50 Game Audio";
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
        nodeName = "alsa_output.usb-Logitech_A50-00.pro-output-0";
        description = "A50 Chat Audio";
        nick = "A50 Chat";
      }
      {
        nodeName = "alsa_output.usb-Logitech_A50-00.pro-output-1";
        description = "A50 Game Audio";
        nick = "A50 Game";
      }
      {
        nodeName = "alsa_input.usb-Logitech_A50-00.pro-input-0";
        description = "A50 Microphone";
        nick = "A50 Mic";
      }
      {
        nodeName = "alsa_input.usb-Logitech_A50-00.pro-input-1";
        description = "A50 Chat Input";
        nick = "A50 Chat Mic";
      }
    ];
    deviceSettings = {
      "usb-Generic_USB_Audio-00" = {
        priority = 50;
        deviceProps = {
          "device.profile" = "HiFi 5+1";
        };
      };
      "usb-Logitech_A50-00" = {
        priority = 51;
        deviceProps = {
          "api.acp.auto-profile" = "false";
          "device.profile" = "pro-audio";
        };
      };
    };
  };
}
