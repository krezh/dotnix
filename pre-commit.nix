{ pkgs, ... }:
{
  check.enable = true;
  settings = {
    hooks = {
      nixfmt.enable = true;
      nixfmt.package = pkgs.nixfmt-rfc-style;
      deadnix.enable = true;
      shellcheck.enable = true;
      check-shebang-scripts-are-executable.enable = true;
      check-case-conflicts.enable = true;
      check-json.enable = true;
      lua-ls.enable = true;
      luacheck.enable = true;
    };
  };
}
