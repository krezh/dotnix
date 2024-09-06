{ ... }: {
  check.enable = true;
  settings = {
    hooks = {
      nixfmt-rfc-style.enable = true;
      deadnix.enable = true;
      shellcheck.enable = true;
      check-shebang-scripts-are-executable.enable = true;
    };
  };
}
