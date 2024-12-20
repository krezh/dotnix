{
  writeShellApplication,
  pkgs,
  ...
}:
writeShellApplication {
  name = "tlk";

  runtimeInputs = with pkgs; [
    gum
  ];

  text = builtins.readFile ./script.sh;
}
