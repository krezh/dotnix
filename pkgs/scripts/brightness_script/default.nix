{
  writeShellApplication,
  brightnessctl,
  dunst,
  ripgrep,
  ...
}:
writeShellApplication {
  name = "brightness_script";

  runtimeInputs = [
    brightnessctl
    dunst
    ripgrep
  ];

  text = builtins.readFile ./brightness.sh;
}
