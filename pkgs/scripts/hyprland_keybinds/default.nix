{writeShellApplication, ...}:
writeShellApplication {
  name = "hyprland_keybinds";

  runtimeInputs = [];

  text = builtins.readFile ./script.sh;
}
