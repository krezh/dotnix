{ inputs, ... }:
{
  imports = [ inputs.noctalia.homeModules.default ];

  programs.noctalia-shell = {
    enable = true;
    systemd.enable = true;
    settings = {
      appLauncher = {
        backgroundOpacity = 1;
        customLaunchPrefix = "";
        customLaunchPrefixEnabled = false;
        enableClipboardHistory = false;
        pinnedExecs = [ ];
        position = "center";
        sortByMostUsed = true;
        terminalCommand = "xterm -e";
        useApp2Unit = false;
      };

      audio = {
        cavaFrameRate = 60;
        mprisBlacklist = [ ];
        preferredPlayer = "";
        visualizerType = "linear";
        volumeOverdrive = false;
        volumeStep = 5;
      };

      bar = {
        backgroundOpacity = 1;
        density = "compact";
        exclusive = true;
        floating = true;
        marginHorizontal = 0.25;
        marginVertical = 0.25;
        monitors = [ ];
        outerCorners = true;
        position = "top";
        showCapsule = false;
        widgets = {
          center = [
            {
              characterCount = 2;
              hideUnoccupied = false;
              id = "Workspace";
              labelMode = "none";
            }
          ];
          left = [
            {
              id = "SystemMonitor";
              showCpuTemp = true;
              showCpuUsage = true;
              showDiskUsage = true;
              showMemoryAsPercent = true;
              showMemoryUsage = true;
              showNetworkStats = false;
              usePrimaryColor = false;
            }
            {
              hideMode = "hidden";
              hideWhenIdle = false;
              id = "MediaMini";
              maxWidth = 145;
              scrollingMode = "always";
              showAlbumArt = false;
              showVisualizer = true;
              useFixedWidth = true;
              visualizerType = "linear";
            }
            {
              colorizeIcons = true;
              hideMode = "hidden";
              id = "ActiveWindow";
              maxWidth = 145;
              scrollingMode = "hover";
              showIcon = false;
              useFixedWidth = false;
            }
          ];
          right = [
            {
              blacklist = [ ];
              colorizeIcons = false;
              drawerEnabled = false;
              favorites = [ ];
              id = "Tray";
            }
            {
              displayMode = "onhover";
              id = "WiFi";
            }
            {
              displayMode = "onhover";
              id = "Bluetooth";
            }
            {
              displayMode = "onhover";
              id = "Battery";
              warningThreshold = 30;
            }
            {
              displayMode = "onhover";
              id = "Volume";
            }
            {
              displayMode = "onhover";
              id = "Brightness";
            }
            {
              id = "NightLight";
            }
            {
              customFont = "";
              formatHorizontal = "HH:mm ddd, MMM dd";
              formatVertical = "HH mm - dd MM";
              id = "Clock";
              useCustomFont = false;
              usePrimaryColor = false;
            }
            {
              hideWhenZero = true;
              id = "NotificationHistory";
              showUnreadBadge = true;
            }
            {
              customIconPath = "";
              icon = "noctalia";
              id = "ControlCenter";
              useDistroLogo = false;
            }
          ];
        };
      };

      dock = {
        enabled = false;
      };

      battery = {
        chargingMode = 0;
      };

      brightness = {
        brightnessStep = 5;
        enableDdcSupport = false;
        enforceMinimum = true;
      };

      colorSchemes = {
        darkMode = true;
        generateTemplatesForPredefined = true;
        manualSunrise = "06:30";
        manualSunset = "18:30";
        matugenSchemeType = "scheme-fruit-salad";
        predefinedScheme = "Tokyo Night";
        schedulingMode = "off";
        useWallpaperColors = false;
      };

      controlCenter = {
        cards = [
          {
            enabled = true;
            id = "profile-card";
          }
          {
            enabled = true;
            id = "shortcuts-card";
          }
          {
            enabled = true;
            id = "audio-card";
          }
          {
            enabled = true;
            id = "weather-card";
          }
          {
            enabled = true;
            id = "media-sysmon-card";
          }
        ];
        position = "close_to_bar_button";
        shortcuts = {
          left = [
            {
              id = "WiFi";
            }
            {
              id = "Bluetooth";
            }
            {
              id = "WallpaperSelector";
            }
          ];
          right = [
            {
              id = "PowerProfile";
            }
            {
              id = "KeepAwake";
            }
            {
              id = "NightLight";
            }
            {
              id = "Notifications";
            }
          ];
        };
      };

      general = {
        animationDisabled = false;
        animationSpeed = 1;
        avatarImage = "/home/krezh/.face";
        compactLockScreen = true;
        dimDesktop = false;
        enableShadows = false;
        forceBlackScreenCorners = false;
        language = "";
        lockOnSuspend = false;
        radiusRatio = 1;
        scaleRatio = 1;
        screenRadiusRatio = 1;
        shadowDirection = "bottom_right";
        shadowOffsetX = 2;
        shadowOffsetY = 3;
        showScreenCorners = false;
      };

      hooks = {
        darkModeChange = "";
        enabled = false;
        wallpaperChange = "";
      };

      location = {
        analogClockInCalendar = false;
        firstDayOfWeek = -1;
        name = "Sweden, Bålsta";
        showCalendarEvents = true;
        showCalendarWeather = true;
        showWeekNumberInCalendar = true;
        use12hourFormat = false;
        useFahrenheit = false;
        weatherEnabled = true;
      };

      network = {
        wifiEnabled = false;
      };

      nightLight = {
        autoSchedule = true;
        dayTemp = "6500";
        enabled = false;
        forced = false;
        manualSunrise = "06:30";
        manualSunset = "18:30";
        nightTemp = "4000";
      };

      notifications = {
        backgroundOpacity = 1;
        criticalUrgencyDuration = 15;
        doNotDisturb = false;
        enabled = true;
        location = "top";
        lowUrgencyDuration = 3;
        monitors = [ ];
        normalUrgencyDuration = 8;
        overlayLayer = true;
        respectExpireTimeout = true;
      };

      osd = {
        autoHideMs = 2000;
        enabled = true;
        location = "right";
        monitors = [ ];
        overlayLayer = true;
      };

      screenRecorder = {
        audioCodec = "opus";
        audioSource = "default_output";
        colorRange = "limited";
        directory = "/home/krezh/Videos";
        frameRate = 60;
        quality = "very_high";
        showCursor = true;
        videoCodec = "h264";
        videoSource = "portal";
      };

      templates = {
        alacritty = false;
        code = false;
        discord = false;
        discord_armcord = false;
        discord_dorion = false;
        discord_equibop = false;
        discord_lightcord = false;
        discord_vesktop = false;
        discord_webcord = false;
        enableUserTemplates = false;
        foot = false;
        fuzzel = false;
        ghostty = false;
        gtk = false;
        kcolorscheme = false;
        kitty = false;
        pywalfox = false;
        qt = false;
        vicinae = false;
        walker = false;
        wezterm = false;
      };

      ui = {
        fontDefault = "Rubik Medium";
        fontDefaultScale = 1.1;
        fontFixed = "JetBrainsMono Nerd Font";
        fontFixedScale = 1.1;
        panelsAttachedToBar = true;
        settingsPanelAttachToBar = false;
        tooltipsEnabled = true;
      };

      wallpaper = {
        defaultWallpaper = "";
        directory = "/home/krezh/Pictures/Wallpapers";
        enableMultiMonitorDirectories = false;
        enabled = false;
        fillColor = "#000000";
        fillMode = "crop";
        monitors = [ ];
        overviewEnabled = true;
        panelPosition = "follow_bar";
        randomEnabled = false;
        randomIntervalSec = 300;
        recursiveSearch = false;
        setWallpaperOnAllMonitors = true;
        transitionDuration = 1500;
        transitionEdgeSmoothness = 0.05;
        transitionType = "random";
      };
    };
  };
}
