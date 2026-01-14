{ inputs, ... }:
{
  imports = [
    inputs.pre-commit-hooks.flakeModule
  ];
  perSystem = {
    pre-commit.check.enable = true;
    pre-commit.settings = {
      hooks = {
        check-shebang-scripts-are-executable.enable = true;
        treefmt.enable = true;
        deadnix.enable = true;
      };
    };
  };
}
