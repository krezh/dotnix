{
  writeShellApplication,
  brightnessctl,
  dunst,
  ripgrep,
  ...
}:
writeShellApplication {
  name = "volume_brightness_script";

  runtimeInputs = [
    brightnessctl
    dunst
    ripgrep
  ];

  text = builtins.readFile ./volume_brightness.sh;
}
