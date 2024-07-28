{
  writeShellApplication,
  wireplumber,
  dunst,
  ripgrep,
  ...
}:
writeShellApplication {
  name = "volume_script";

  runtimeInputs = [
    wireplumber
    dunst
    ripgrep
  ];

  text = builtins.readFile ./volume.sh;
}
