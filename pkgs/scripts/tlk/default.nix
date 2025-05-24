{
  writeShellApplication,
  pkgs,
  ...
}:
writeShellApplication {
  name = "tlk";

  runtimeInputs = with pkgs; [
    gum
    jq
    yq-go
  ];

  text = builtins.readFile ./script.sh;
}
