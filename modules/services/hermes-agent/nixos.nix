{ inputs, ... }:
{
  flake.modules.nixos.hermes-agent =
    { config, ... }:
    {
      imports = [ inputs.hermes-agent.nixosModules.default ];

      users.users.krezh.extraGroups = [ "hermes" ];

      sops.secrets."hermes-env" = {
        format = "yaml";
      };

      services.hermes-agent = {
        enable = true;
        settings = {
          model = {
            base_url = "https://api.anthropic.com";
            default = "anthropic/claude-sonnet-4-6";
          };
          toolsets = [ "all" ];
          max_turns = 100;
          terminal = {
            backend = "local";
            cwd = ".";
            timeout = 180;
          };
          compression = {
            enabled = true;
            threshold = 0.85;
            # summary_model = "google/gemini-3-flash-preview";
          };
          memory = {
            memory_enabled = true;
            user_profile_enabled = true;
          };
          display = {
            compact = false;
            show_cost = true;
          };
          checkpoints = {
            enabled = true;
            max_snapshots = 50;
          };
          agent = {
            max_turns = 60;
            verbose = false;
          };
        };
        environmentFiles = [ config.sops.secrets."hermes-env".path ];
        documents = {
          # "USER.md" = ./documents/USER.md;
        };
        extraDependencyGroups = [ "anthropic" ];
        addToSystemPackages = true;
        restartSec = 5;
      };
    };
}
