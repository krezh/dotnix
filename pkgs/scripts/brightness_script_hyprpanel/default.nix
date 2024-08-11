{ writeShellApplication, brightnessctl, ... }:
writeShellApplication {
  name = "brightness_script_hyprpanel";

  runtimeInputs = [ brightnessctl ];

  text = builtins.readFile ./brightness.sh;
}
