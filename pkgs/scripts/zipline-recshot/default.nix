{
  writeShellApplication,
  pkgs,
  ...
}:
writeShellApplication {
  name = "zipline-recshot";

  runtimeInputs = with pkgs; [
    curl
    jq
    wl-clipboard
    grim
    slurp
    wl-screenrec
  ];

  text = builtins.readFile ./script.sh;
}
