{ ... }:
{
  modules.shell.mise = {
    enable = false;
    config = {
      python_venv_auto_create = true;
      status = {
        missing_tools = "always";
        show_env = false;
        show_tools = false;
      };
    };
  };
}
