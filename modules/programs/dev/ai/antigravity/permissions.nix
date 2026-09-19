{
  flake.modules.homeManager.ai = {
    programs.antigravity-cli = {
      permissions = {
        allow = [
          # Safe git commands
          "command(git)"

          # Safe jj commands
          "command(jj)"

          # Safe Nix commands
          "command(nix)"
          "command(nh search)"
          "command(nh os build)"
          "command(nixos-rebuild build)"

          # Programming language tools
          "command(cargo)"
          "command(go)"

          # Safe filesystem operations
          "command(ls)"
          "command(find)"
          "command(grep)"
          "command(rg)"
          "command(cat)"
          "command(head)"
          "command(tail)"
          "command(mkdir)"
          "command(chmod)"

          # Safe system info commands
          "command(systemctl list-units)"
          "command(systemctl list-timers)"
          "command(systemctl status)"
          "command(journalctl)"
          "command(dmesg)"
          "command(env)"
          "command(agy --version)"

          # Audio system
          "command(pactl list)"
          "command(pw-top)"

          # GitHub CLI read-only
          "command(gh search)"
          "command(gh api)"

          # Kubernetes read-only
          "command(kubectl get)"
          "command(kubectl logs)"
          "command(kubectl describe)"

          # Security / lint tools
          "command(shellcheck)"
          "command(zizmor)"

          # Safe web fetch from trusted domains
          "read_url(wiki.hyprland.org)"
          "read_url(wiki.hypr.land)"
          "read_url(github.com)"
          "read_url(raw.githubusercontent.com)"
          "read_url(*renovatebot.com)"
        ];

        deny = [
          "command(sudo)"
          "command(nh os switch)"
          "command(kubectl get secret)"
          "command(kubectl get secrets)"
          "read(**/.secret*)"
          "read(**/secret)"
          "read(**/secret.*)"
          "read(**/.decrypted~secrets.sops.*)"
        ];
      };
    };
  };
}
