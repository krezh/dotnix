{ inputs, ... }:
{
  imports = [
    (inputs.import-tree ./desktop)
    (inputs.import-tree ./shell)
  ];
}
