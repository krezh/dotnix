{
  writeShellApplication,
  niri,
  jq,
  bc,
  libnotify,
  ...
}:
writeShellApplication {
  name = "niri-shutdown";

  runtimeInputs = [
    niri
    jq
    bc
    libnotify
  ];

  text = builtins.readFile ./shutdown.sh;
}
