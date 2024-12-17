{ ... }:
{
  check.enable = true;
  settings = {
    hooks = {
      check-shebang-scripts-are-executable.enable = true;
      treefmt.enable = true;
    };
  };
}
