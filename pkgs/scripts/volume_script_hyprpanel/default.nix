{
  writeShellApplication,
  wireplumber,
  ...
}:
writeShellApplication {
  name = "volume_script_hyprpanel";

  runtimeInputs = [wireplumber];

  text = builtins.readFile ./volume.sh;
}
