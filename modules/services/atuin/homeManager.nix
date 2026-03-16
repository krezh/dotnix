{
  flake.modules.homeManager.atuin = {
    programs.atuin = {
      enable = true;
      enableBashIntegration = true;
      enableFishIntegration = true;
      enableZshIntegration = true;
      enableNushellIntegration = true;
      daemon.enable = true;
      flags = [ "--disable-up-arrow" ];
      settings = {
        sync_address = "https://sh.talos.plexuz.xyz";
        style = "compact";
        workspaces = true;
        auto_sync = true;
        sync_frequency = "1m";
        search_mode = "fuzzy";
        store_failed = false;
        show_preview = true;
        filter_mode = "workspace";
        update_check = false;
        sync.records = true;
        history_filter = [
          "^(sudo reboot)$"
          "^(reboot)$"
        ];
      };
    };
  };
}
