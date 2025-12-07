{
  config,
  pkgs,
  inputs,
  ...
}:
{
  imports = [
    inputs.buildbot-nix.nixosModules.buildbot-master
    ../../images/buildbot-master.nix
  ];

  networking.hostName = "buildbot-master";

  # Auto-upgrade from main branch
  system.autoUpgrade = {
    enable = true;
    flake = "github:krezh/nix-config";
    dates = "hourly";
    allowReboot = false;
  };

  services.buildbot-nix.master = {
    enable = true;
    domain = "buildbot.plexuz.xyz";

    # Workers configuration from secrets
    # For KubeVirt pool, create a JSON with workers named buildbot-worker-0, buildbot-worker-1, etc.
    # Format: [{ "name": "buildbot-worker-0", "pass": "password", "cores": 8 }, ...]
    workersFile = config.age.secrets.buildbot-workers.path or "/var/lib/secrets/buildbot-workers.json";

    # Admin users who can reload projects
    admins = [ "krezh" ];

    # GitHub configuration
    authBackend = "github";
    github = {
      appId = 0; # TODO: Set GitHub App ID
      appSecretKeyFile =
        config.age.secrets.github-buildbot-app-secret.path or "/var/lib/secrets/github-app-secret";
      webhookSecretFile =
        config.age.secrets.github-buildbot-webhook-secret.path or "/var/lib/secrets/github-webhook-secret";
      oauthId = ""; # TODO: Set OAuth client ID
      oauthSecretFile =
        config.age.secrets.github-buildbot-oauth-secret.path or "/var/lib/secrets/github-oauth-secret";
      topic = "buildbot-nix";
    };

    # Evaluation settings
    evalWorkerCount = 4;
    evalMaxMemorySize = 2048;
  };

  # SSH for administration
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "prohibit-password";
      PasswordAuthentication = false;
    };
  };

  users.users.root.openssh.authorizedKeys.keys = [
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIANNodE0rg2XalK+tfsqfPwLdBRJIx15IjGwkr5Bud+W"
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEMe4X4oNA8PRUHrOk5RIrpxpzzcBvJyQa8PyaQj3BPp"
  ];

  networking.firewall.allowedTCPPorts = [
    80
    443
  ];

  nix.settings = {
    experimental-features = [
      "nix-command"
      "flakes"
    ];
    trusted-users = [ "buildbot" ];
  };

  environment.systemPackages = with pkgs; [
    neovim
    gitMinimal
  ];

  time.timeZone = "Europe/Stockholm";
  console.keyMap = "sv-latin1";
  system.stateVersion = "24.05";
}
