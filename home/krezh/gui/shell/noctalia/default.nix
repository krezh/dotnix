{ inputs, ... }:
{
  imports = [ inputs.noctalia.homeModules.default ];

  programs.noctalia-shell = {
    enable = true;
    systemd.enable = true;
    settings = {
      appLauncher = {
        customLaunchPrefix = "";
        customLaunchPrefixEnabled = false;
        enableClipPreview = true;
        enableClipboardHistory = false;
        iconMode = "tabler";
        ignoreMouseInput = false;
        pinnedExecs = [ ];
        position = "center";
        showCategories = true;
        sortByMostUsed = true;
        terminalCommand = "xterm -e";
        useApp2Unit = false;
        viewMode = "list";
      };
      audio = {
        cavaFrameRate = 60;
        externalMixer = "pwvucontrol || pavucontrol";
        mprisBlacklist = [ ];
        preferredPlayer = "";
        visualizerType = "linear";
        volumeOverdrive = false;
        volumeStep = 5;
      };
      bar = {
        backgroundOpacity = 0;
        capsuleOpacity = 1;
        density = "default";
        exclusive = true;
        floating = false;
        marginHorizontal = 0.25;
        marginVertical = 0.25;
        monitors = [ ];
        outerCorners = false;
        position = "top";
        showCapsule = false;
        showOutline = false;
        useSeparateOpacity = false;
        widgets = {
          center = [
            {
              characterCount = 2;
              colorizeIcons = false;
              enableScrollWheel = true;
              followFocusedScreen = false;
              groupedBorderOpacity = 1;
              hideUnoccupied = true;
              iconScale = 0.8;
              id = "Workspace";
              labelMode = "index";
              showApplications = false;
              showLabelsOnlyWhenOccupied = true;
              unfocusedIconsOpacity = 1;
            }
          ];
          left = [
            {
              compactMode = false;
              diskPath = "/";
              id = "SystemMonitor";
              showCpuTemp = true;
              showCpuUsage = true;
              showDiskUsage = true;
              showGpuTemp = true;
              showMemoryAsPercent = true;
              showMemoryUsage = true;
              showNetworkStats = false;
              useMonospaceFont = true;
              usePrimaryColor = false;
            }
            {
              hideMode = "hidden";
              hideWhenIdle = false;
              id = "MediaMini";
              maxWidth = 300;
              scrollingMode = "always";
              showAlbumArt = false;
              showArtistFirst = true;
              showProgressRing = true;
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
              colorizeIcons = true;
              drawerEnabled = false;
              hidePassive = false;
              id = "Tray";
              pinned = [ ];
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
              displayMode = "alwaysShow";
              hideIfNotDetected = true;
              id = "Battery";
              showNoctaliaPerformance = false;
              showPowerProfiles = false;
              warningThreshold = 30;
            }
            {
              displayMode = "alwaysShow";
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
              tooltipFormat = "HH:mm ddd, MMM dd";
              useCustomFont = false;
              usePrimaryColor = false;
            }
            {
              hideWhenZero = false;
              id = "NotificationHistory";
              showUnreadBadge = true;
            }
            {
              colorizeDistroLogo = false;
              colorizeSystemIcon = "primary";
              customIconPath = "";
              enableColorization = true;
              icon = "noctalia";
              id = "ControlCenter";
              useDistroLogo = true;
            }
          ];
        };
      };
      brightness = {
        brightnessStep = 5;
        enableDdcSupport = false;
        enforceMinimum = true;
      };
      calendar = {
        cards = [
          {
            enabled = true;
            id = "calendar-header-card";
          }
          {
            enabled = true;
            id = "calendar-month-card";
          }
          {
            enabled = true;
            id = "timer-card";
          }
          {
            enabled = true;
            id = "weather-card";
          }
        ];
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
          {
            enabled = true;
            id = "brightness-card";
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
              id = "KeepAwake";
            }
          ];
          right = [
            {
              id = "PowerProfile";
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
      desktopWidgets = {
        enabled = true;
        gridSnap = false;
        monitorWidgets = [
          {
            name = "DP-1";
            widgets = [
              {
                hideMode = "visible";
                id = "MediaPlayer";
                roundedCorners = true;
                showAlbumArt = true;
                showBackground = true;
                showButtons = true;
                showVisualizer = true;
                visualizerType = "linear";
                x = 100;
                y = 200;
              }
              {
                id = "Weather";
                showBackground = true;
                x = 100;
                y = 300;
              }
              {
                clockStyle = "digital";
                format = "HH:mm\\nd MMMM yyyy";
                id = "Clock";
                showBackground = true;
                useCustomFont = false;
                usePrimaryColor = false;
                x = 50;
                y = 50;
              }
            ];
          }
        ];
      };
      dock = {
        animationSpeed = 1;
        backgroundOpacity = 1;
        colorizeIcons = false;
        deadOpacity = 0.6;
        displayMode = "auto_hide";
        enabled = false;
        floatingRatio = 1;
        inactiveIndicators = false;
        monitors = [ ];
        onlySameOutput = true;
        pinnedApps = [ ];
        pinnedStatic = false;
        size = 1;
      };
      general = {
        allowPanelsOnScreenWithoutBar = true;
        animationDisabled = false;
        animationSpeed = 1;
        avatarImage = "/home/krezh/.face";
        boxRadiusRatio = 1;
        compactLockScreen = true;
        dimmerOpacity = 0.2;
        enableShadows = true;
        forceBlackScreenCorners = false;
        iRadiusRatio = 1;
        language = "";
        lockOnSuspend = false;
        radiusRatio = 1;
        scaleRatio = 1;
        screenRadiusRatio = 1;
        shadowDirection = "bottom_right";
        shadowOffsetX = 2;
        shadowOffsetY = 3;
        showHibernateOnLockScreen = false;
        showScreenCorners = false;
        showSessionButtonsOnLockScreen = true;
      };
      hooks = {
        darkModeChange = "";
        enabled = false;
        performanceModeDisabled = "";
        performanceModeEnabled = "";
        screenLock = "";
        screenUnlock = "";
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
        weatherShowEffects = true;
      };
      network = {
        wifiEnabled = true;
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
        backgroundOpacity = 0.98;
        criticalUrgencyDuration = 15;
        enableKeyboardLayoutToast = true;
        enabled = true;
        location = "top_right";
        lowUrgencyDuration = 3;
        monitors = [ ];
        normalUrgencyDuration = 8;
        overlayLayer = true;
        respectExpireTimeout = true;
        saveToHistory = {
          critical = true;
          low = true;
          normal = true;
        };
        sounds = {
          criticalSoundFile = "";
          enabled = false;
          excludedApps = "discord,firefox,chrome,chromium,edge";
          lowSoundFile = "";
          normalSoundFile = "";
          separateSounds = false;
          volume = 0.5;
        };
      };
      osd = {
        autoHideMs = 2000;
        backgroundOpacity = 1;
        enabled = true;
        enabledTypes = [
          0
          1
          2
          4
          3
        ];
        location = "right";
        monitors = [ ];
        overlayLayer = true;
      };
      screenRecorder = {
        audioCodec = "opus";
        audioSource = "default_output";
        colorRange = "limited";
        copyToClipboard = false;
        directory = "/home/krezh/Videos";
        frameRate = 60;
        quality = "very_high";
        showCursor = true;
        videoCodec = "h264";
        videoSource = "portal";
      };
      sessionMenu = {
        countdownDuration = 10000;
        enableCountdown = true;
        largeButtonsStyle = true;
        position = "center";
        powerOptions = [
          {
            action = "lock";
            command = "";
            countdownEnabled = true;
            enabled = true;
          }
          {
            action = "suspend";
            command = "";
            countdownEnabled = true;
            enabled = true;
          }
          {
            action = "hibernate";
            command = "";
            countdownEnabled = true;
            enabled = true;
          }
          {
            action = "reboot";
            command = "";
            countdownEnabled = true;
            enabled = true;
          }
          {
            action = "logout";
            command = "";
            countdownEnabled = true;
            enabled = true;
          }
          {
            action = "shutdown";
            command = "";
            countdownEnabled = true;
            enabled = true;
          }
        ];
        showHeader = true;
        showNumberLabels = true;
      };
      settingsVersion = 35;
      systemMonitor = {
        cpuCriticalThreshold = 90;
        cpuPollingInterval = 3000;
        cpuWarningThreshold = 80;
        criticalColor = "";
        diskCriticalThreshold = 90;
        diskPath = "/";
        diskPollingInterval = 3000;
        diskWarningThreshold = 80;
        enableDgpuMonitoring = true;
        externalMonitor = "resources || missioncenter || jdsystemmonitor || corestats || system-monitoring-center || gnome-system-monitor || plasma-systemmonitor || mate-system-monitor || ukui-system-monitor || deepin-system-monitor || pantheon-system-monitor";
        gpuCriticalThreshold = 90;
        gpuPollingInterval = 3000;
        gpuWarningThreshold = 80;
        memCriticalThreshold = 90;
        memPollingInterval = 3000;
        memWarningThreshold = 80;
        networkPollingInterval = 3000;
        tempCriticalThreshold = 90;
        tempPollingInterval = 3000;
        tempWarningThreshold = 80;
        useCustomColors = false;
        warningColor = "";
      };
      templates = {
        alacritty = false;
        cava = false;
        code = false;
        discord = false;
        emacs = false;
        enableUserTemplates = false;
        foot = false;
        fuzzel = false;
        ghostty = false;
        gtk = false;
        helix = false;
        hyprland = false;
        kcolorscheme = false;
        kitty = false;
        mango = false;
        niri = false;
        pywalfox = false;
        qt = false;
        spicetify = false;
        telegram = false;
        vicinae = false;
        walker = false;
        wezterm = false;
        yazi = false;
        zed = false;
      };
      ui = {
        bluetoothDetailsViewMode = "grid";
        bluetoothHideUnnamedDevices = false;
        fontDefault = "Rubik";
        fontDefaultScale = 1.1;
        fontFixed = "JetBrainsMono Nerd Font";
        fontFixedScale = 1.1;
        panelBackgroundOpacity = 0.93;
        panelsAttachedToBar = true;
        settingsPanelMode = "centered";
        tooltipsEnabled = true;
        wifiDetailsViewMode = "grid";
      };
      wallpaper = {
        directory = "/home/krezh/Pictures/Wallpapers";
        enableMultiMonitorDirectories = false;
        enabled = false;
        fillColor = "#000000";
        fillMode = "crop";
        hideWallpaperFilenames = false;
        monitorDirectories = [ ];
        overviewEnabled = true;
        panelPosition = "follow_bar";
        randomEnabled = false;
        randomIntervalSec = 300;
        recursiveSearch = false;
        setWallpaperOnAllMonitors = true;
        transitionDuration = 1500;
        transitionEdgeSmoothness = 0.05;
        transitionType = "random";
        useWallhaven = false;
        wallhavenApiKey = "";
        wallhavenCategories = "111";
        wallhavenOrder = "desc";
        wallhavenPurity = "100";
        wallhavenQuery = "";
        wallhavenRatios = "";
        wallhavenResolutionHeight = "";
        wallhavenResolutionMode = "atleast";
        wallhavenResolutionWidth = "";
        wallhavenSorting = "relevance";
        wallpaperChangeMode = "random";
      };
    };
  };
}
