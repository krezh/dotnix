{ inputs, ... }:
{
  flake.modules.homeManager.desktop-shell =
    { ... }:
    {
      imports = [ inputs.noctalia.homeModules.default ];

      programs.noctalia-shell = {
        enable = true;
        systemd.enable = true;
        settings = {
          settingsVersion = 58;
          bar = {
            barType = "floating";
            position = "top";
            monitors = [

            ];
            density = "default";
            showOutline = false;
            showCapsule = false;
            capsuleOpacity = 1;
            capsuleColorKey = "none";
            widgetSpacing = 6;
            contentPadding = 2;
            fontScale = 1;
            enableExclusionZoneInset = true;
            backgroundOpacity = 0.9;
            useSeparateOpacity = true;
            floating = true;
            marginVertical = 5;
            marginHorizontal = 10;
            frameThickness = 8;
            frameRadius = 12;
            outerCorners = false;
            hideOnOverview = false;
            displayMode = "always_visible";
            autoHideDelay = 500;
            autoShowDelay = 150;
            showOnWorkspaceSwitch = true;
            widgets = {
              left = [
                {
                  compactMode = false;
                  diskPath = "/";
                  iconColor = "none";
                  id = "SystemMonitor";
                  showCpuCores = false;
                  showCpuFreq = false;
                  showCpuTemp = true;
                  showCpuUsage = true;
                  showDiskAvailable = false;
                  showDiskUsage = true;
                  showDiskUsageAsPercent = false;
                  showGpuTemp = true;
                  showLoadAverage = false;
                  showMemoryAsPercent = true;
                  showMemoryUsage = true;
                  showNetworkStats = false;
                  showSwapUsage = false;
                  textColor = "none";
                  useMonospaceFont = false;
                  usePadding = false;
                }
                {
                  colorizeIcons = true;
                  hideMode = "hidden";
                  id = "ActiveWindow";
                  maxWidth = 145;
                  scrollingMode = "hover";
                  showIcon = false;
                  textColor = "none";
                  useFixedWidth = false;
                }
                {
                  compactMode = false;
                  hideMode = "hidden";
                  hideWhenIdle = true;
                  id = "MediaMini";
                  maxWidth = 300;
                  panelShowAlbumArt = true;
                  scrollingMode = "always";
                  showAlbumArt = false;
                  showArtistFirst = true;
                  showProgressRing = true;
                  showVisualizer = true;
                  textColor = "none";
                  useFixedWidth = true;
                  visualizerType = "mirrored";
                }
              ];
              center = [
                {
                  characterCount = 2;
                  colorizeIcons = false;
                  emptyColor = "secondary";
                  enableScrollWheel = true;
                  focusedColor = "primary";
                  followFocusedScreen = false;
                  fontWeight = "bold";
                  groupedBorderOpacity = 1;
                  hideUnoccupied = true;
                  iconScale = 0.8;
                  id = "Workspace";
                  labelMode = "index";
                  occupiedColor = "secondary";
                  pillSize = 0.6;
                  showApplications = false;
                  showApplicationsHover = false;
                  showBadge = true;
                  showLabelsOnlyWhenOccupied = true;
                  unfocusedIconsOpacity = 1;
                }
              ];
              right = [
                {
                  blacklist = [

                  ];
                  chevronColor = "none";
                  colorizeIcons = true;
                  drawerEnabled = false;
                  hidePassive = false;
                  id = "Tray";
                  pinned = [

                  ];
                }
                {
                  displayMode = "onhover";
                  iconColor = "none";
                  id = "Network";
                  textColor = "none";
                }
                {
                  displayMode = "onhover";
                  iconColor = "none";
                  id = "Bluetooth";
                  textColor = "none";
                }
                {
                  deviceNativePath = "";
                  displayMode = "alwaysShow";
                  hideIfIdle = false;
                  hideIfNotDetected = true;
                  id = "Battery";
                  showNoctaliaPerformance = false;
                  showPowerProfiles = false;
                }
                {
                  displayMode = "alwaysShow";
                  iconColor = "none";
                  id = "Volume";
                  middleClickCommand = "pwvucontrol || pavucontrol";
                  textColor = "none";
                }
                {
                  applyToAllMonitors = false;
                  displayMode = "onhover";
                  iconColor = "none";
                  id = "Brightness";
                  textColor = "none";
                }
                {
                  iconColor = "none";
                  id = "NightLight";
                }
                {
                  clockColor = "none";
                  customFont = "";
                  formatHorizontal = "HH:mm ddd, MMM dd";
                  formatVertical = "HH mm - dd MM";
                  id = "Clock";
                  tooltipFormat = "HH:mm ddd, MMM dd";
                  useCustomFont = false;
                }
                {
                  hideWhenZero = false;
                  hideWhenZeroUnread = false;
                  iconColor = "none";
                  id = "NotificationHistory";
                  showUnreadBadge = true;
                  unreadBadgeColor = "primary";
                }
              ];
            };
            mouseWheelAction = "none";
            reverseScroll = false;
            mouseWheelWrap = true;
            middleClickAction = "none";
            middleClickFollowMouse = false;
            middleClickCommand = "";
            rightClickAction = "controlCenter";
            rightClickFollowMouse = true;
            rightClickCommand = "";
            screenOverrides = [

            ];
          };
          general = {
            avatarImage = "/home/krezh/.face";
            dimmerOpacity = 0.2;
            showScreenCorners = false;
            forceBlackScreenCorners = false;
            scaleRatio = 1;
            radiusRatio = 1;
            iRadiusRatio = 1;
            boxRadiusRatio = 1;
            screenRadiusRatio = 1;
            animationSpeed = 1;
            animationDisabled = false;
            compactLockScreen = true;
            lockScreenAnimations = true;
            lockOnSuspend = false;
            showSessionButtonsOnLockScreen = true;
            showHibernateOnLockScreen = false;
            enableLockScreenMediaControls = false;
            enableShadows = true;
            enableBlurBehind = true;
            shadowDirection = "bottom_right";
            shadowOffsetX = 2;
            shadowOffsetY = 3;
            language = "";
            allowPanelsOnScreenWithoutBar = true;
            showChangelogOnStartup = true;
            telemetryEnabled = false;
            enableLockScreenCountdown = true;
            lockScreenCountdownDuration = 10000;
            autoStartAuth = false;
            allowPasswordWithFprintd = false;
            clockStyle = "custom";
            clockFormat = "hh\\nmm";
            passwordChars = false;
            lockScreenMonitors = [

            ];
            lockScreenBlur = 0;
            lockScreenTint = 0;
            keybinds = {
              keyUp = [
                "Up"
              ];
              keyDown = [
                "Down"
              ];
              keyLeft = [
                "Left"
              ];
              keyRight = [
                "Right"
              ];
              keyEnter = [
                "Return"
                "Enter"
              ];
              keyEscape = [
                "Esc"
              ];
              keyRemove = [
                "Del"
              ];
            };
            reverseScroll = false;
          };
          ui = {
            fontDefault = "Rubik";
            fontFixed = "JetBrainsMono Nerd Font";
            fontDefaultScale = 1.1;
            fontFixedScale = 1.1;
            tooltipsEnabled = true;
            scrollbarAlwaysVisible = true;
            boxBorderEnabled = false;
            panelBackgroundOpacity = 0.93;
            translucentWidgets = false;
            panelsAttachedToBar = true;
            settingsPanelMode = "centered";
            settingsPanelSideBarCardStyle = false;
          };
          location = {
            name = "Sweden, Bålsta";
            weatherEnabled = true;
            weatherShowEffects = true;
            useFahrenheit = false;
            use12hourFormat = false;
            showWeekNumberInCalendar = true;
            showCalendarEvents = true;
            showCalendarWeather = true;
            analogClockInCalendar = false;
            firstDayOfWeek = 1;
            hideWeatherTimezone = false;
            hideWeatherCityName = false;
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
                id = "weather-card";
              }
            ];
          };
          wallpaper = {
            enabled = false;
            overviewEnabled = true;
            directory = "/home/krezh/Pictures/Wallpapers";
            monitorDirectories = [

            ];
            enableMultiMonitorDirectories = false;
            showHiddenFiles = false;
            viewMode = "single";
            setWallpaperOnAllMonitors = true;
            fillMode = "crop";
            fillColor = "#000000";
            useSolidColor = false;
            solidColor = "#1a1a2e";
            automationEnabled = false;
            wallpaperChangeMode = "random";
            randomIntervalSec = 300;
            transitionDuration = 1500;
            transitionType = "random";
            skipStartupTransition = false;
            transitionEdgeSmoothness = 0.05;
            panelPosition = "follow_bar";
            hideWallpaperFilenames = false;
            overviewBlur = 0.4;
            overviewTint = 0.6;
            useWallhaven = false;
            wallhavenQuery = "";
            wallhavenSorting = "relevance";
            wallhavenOrder = "desc";
            wallhavenCategories = "111";
            wallhavenPurity = "100";
            wallhavenRatios = "";
            wallhavenApiKey = "";
            wallhavenResolutionMode = "atleast";
            wallhavenResolutionWidth = "";
            wallhavenResolutionHeight = "";
            sortOrder = "name";
            favorites = [

            ];
          };
          appLauncher = {
            enableClipboardHistory = false;
            autoPasteClipboard = false;
            enableClipPreview = true;
            clipboardWrapText = true;
            clipboardWatchTextCommand = "wl-paste --type text --watch cliphist store";
            clipboardWatchImageCommand = "wl-paste --type image --watch cliphist store";
            position = "center";
            pinnedApps = [

            ];
            sortByMostUsed = true;
            terminalCommand = "xterm -e";
            customLaunchPrefixEnabled = false;
            customLaunchPrefix = "";
            viewMode = "list";
            showCategories = true;
            iconMode = "tabler";
            showIconBackground = false;
            enableSettingsSearch = true;
            enableWindowsSearch = true;
            enableSessionSearch = true;
            ignoreMouseInput = false;
            screenshotAnnotationTool = "";
            overviewLayer = false;
            density = "default";
          };
          controlCenter = {
            position = "close_to_bar_button";
            diskPath = "/";
            shortcuts = {
              left = [
                {
                  id = "Network";
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
          };
          systemMonitor = {
            cpuWarningThreshold = 80;
            cpuCriticalThreshold = 90;
            tempWarningThreshold = 80;
            tempCriticalThreshold = 90;
            gpuWarningThreshold = 80;
            gpuCriticalThreshold = 90;
            memWarningThreshold = 80;
            memCriticalThreshold = 90;
            swapWarningThreshold = 80;
            swapCriticalThreshold = 90;
            diskWarningThreshold = 80;
            diskCriticalThreshold = 90;
            diskAvailWarningThreshold = 20;
            diskAvailCriticalThreshold = 10;
            batteryWarningThreshold = 20;
            batteryCriticalThreshold = 5;
            enableDgpuMonitoring = true;
            useCustomColors = false;
            warningColor = "#c6a0f6";
            criticalColor = "#f38ba8";
            externalMonitor = "resources || missioncenter || jdsystemmonitor || corestats || system-monitoring-center || gnome-system-monitor || plasma-systemmonitor || mate-system-monitor || ukui-system-monitor || deepin-system-monitor || pantheon-system-monitor";
          };
          noctaliaPerformance = {
            disableWallpaper = true;
            disableDesktopWidgets = true;
          };
          dock = {
            enabled = true;
            position = "bottom";
            displayMode = "auto_hide";
            dockType = "attached";
            backgroundOpacity = 1;
            floatingRatio = 1;
            size = 1;
            onlySameOutput = true;
            monitors = [

            ];
            pinnedApps = [

            ];
            colorizeIcons = false;
            showLauncherIcon = false;
            launcherPosition = "end";
            launcherIconColor = "none";
            pinnedStatic = false;
            inactiveIndicators = true;
            groupApps = true;
            groupContextMenuMode = "extended";
            groupClickAction = "cycle";
            groupIndicatorStyle = "dots";
            deadOpacity = 0.6;
            animationSpeed = 1;
            sitOnFrame = false;
            showDockIndicator = false;
            indicatorThickness = 3;
            indicatorColor = "primary";
            indicatorOpacity = 0.6;
          };
          network = {
            wifiEnabled = false;
            airplaneModeEnabled = false;
            bluetoothRssiPollingEnabled = false;
            bluetoothRssiPollIntervalMs = 10000;
            networkPanelView = "wifi";
            wifiDetailsViewMode = "grid";
            bluetoothDetailsViewMode = "grid";
            bluetoothHideUnnamedDevices = false;
            disableDiscoverability = false;
            bluetoothAutoConnect = true;
          };
          sessionMenu = {
            enableCountdown = true;
            countdownDuration = 10000;
            position = "center";
            showHeader = true;
            showKeybinds = true;
            largeButtonsStyle = true;
            largeButtonsLayout = "grid";
            powerOptions = [
              {
                action = "lock";
                command = "";
                countdownEnabled = true;
                enabled = true;
                keybind = "1";
              }
              {
                action = "suspend";
                command = "";
                countdownEnabled = true;
                enabled = true;
                keybind = "2";
              }
              {
                action = "hibernate";
                command = "";
                countdownEnabled = true;
                enabled = true;
                keybind = "3";
              }
              {
                action = "reboot";
                command = "";
                countdownEnabled = true;
                enabled = true;
                keybind = "4";
              }
              {
                action = "logout";
                command = "";
                countdownEnabled = true;
                enabled = true;
                keybind = "5";
              }
              {
                action = "shutdown";
                command = "";
                countdownEnabled = true;
                enabled = true;
                keybind = "6";
              }
              {
                action = "userspaceReboot";
                command = "";
                countdownEnabled = true;
                enabled = false;
                keybind = "";
              }
              {
                action = "rebootToUefi";
                command = "";
                countdownEnabled = true;
                enabled = true;
                keybind = "7";
              }
            ];
          };
          notifications = {
            enabled = true;
            enableMarkdown = true;
            density = "default";
            monitors = [

            ];
            location = "top_right";
            overlayLayer = true;
            backgroundOpacity = 0.98;
            respectExpireTimeout = true;
            lowUrgencyDuration = 3;
            normalUrgencyDuration = 8;
            criticalUrgencyDuration = 15;
            clearDismissed = true;
            saveToHistory = {
              low = true;
              normal = true;
              critical = true;
            };
            sounds = {
              enabled = false;
              volume = 0.5;
              separateSounds = false;
              criticalSoundFile = "";
              normalSoundFile = "";
              lowSoundFile = "";
              excludedApps = "discord,firefox,chrome,chromium,edge";
            };
            enableMediaToast = false;
            enableKeyboardLayoutToast = true;
            enableBatteryToast = true;
          };
          osd = {
            enabled = true;
            location = "right";
            autoHideMs = 2000;
            overlayLayer = true;
            backgroundOpacity = 1;
            enabledTypes = [
              0
              1
              2
              4
              3
              null
              null
            ];
            monitors = [

            ];
          };
          audio = {
            volumeStep = 5;
            volumeOverdrive = false;
            spectrumFrameRate = 60;
            visualizerType = "linear";
            mprisBlacklist = [

            ];
            preferredPlayer = "";
            volumeFeedback = false;
            volumeFeedbackSoundFile = "";
          };
          brightness = {
            brightnessStep = 5;
            enforceMinimum = true;
            enableDdcSupport = false;
            backlightDeviceMappings = [

            ];
          };
          colorSchemes = {
            useWallpaperColors = false;
            predefinedScheme = "Tokyo Night";
            darkMode = true;
            schedulingMode = "off";
            manualSunrise = "06:30";
            manualSunset = "18:30";
            generationMethod = "fruit-salad";
            monitorForColors = "";
          };
          templates = {
            activeTemplates = [

            ];
            enableUserTheming = false;
          };
          nightLight = {
            enabled = false;
            forced = false;
            autoSchedule = true;
            nightTemp = "4000";
            dayTemp = "6500";
            manualSunrise = "06:30";
            manualSunset = "18:30";
          };
          hooks = {
            enabled = false;
            wallpaperChange = "";
            darkModeChange = "";
            screenLock = "";
            screenUnlock = "";
            performanceModeEnabled = "";
            performanceModeDisabled = "";
            startup = "";
            session = "";
          };
          plugins = {
            autoUpdate = false;
          };
          idle = {
            enabled = false;
            screenOffTimeout = 600;
            lockTimeout = 660;
            suspendTimeout = 1800;
            fadeDuration = 5;
            screenOffCommand = "";
            lockCommand = "";
            suspendCommand = "";
            resumeScreenOffCommand = "";
            resumeLockCommand = "";
            resumeSuspendCommand = "";
            customCommands = "[]";
          };
          desktopWidgets = {
            enabled = true;
            overviewEnabled = true;
            gridSnap = true;
            gridSnapScale = false;
            monitorWidgets = [
              {
                name = "DP-1";
                widgets = [
                  {
                    diskPath = "/";
                    id = "SystemStat";
                    layout = "bottom";
                    roundedCorners = true;
                    showBackground = true;
                    statType = "CPU";
                    x = 120;
                    y = 120;
                  }
                  {
                    hideMode = "visible";
                    id = "MediaPlayer";
                    roundedCorners = true;
                    showAlbumArt = true;
                    showBackground = true;
                    showButtons = true;
                    showVisualizer = true;
                    visualizerType = "linear";
                    x = 720;
                    y = 120;
                  }
                  {
                    id = "Weather";
                    roundedCorners = true;
                    showBackground = true;
                    x = 1160;
                    y = 120;
                  }
                  {
                    diskPath = "/";
                    id = "SystemStat";
                    layout = "bottom";
                    roundedCorners = true;
                    showBackground = true;
                    statType = "Memory";
                    x = 120;
                    y = 280;
                  }
                  {
                    diskPath = "/";
                    id = "SystemStat";
                    layout = "bottom";
                    roundedCorners = true;
                    scale = 1;
                    showBackground = true;
                    statType = "Network";
                    x = 120;
                    y = 440;
                  }
                  {
                    diskPath = "/home";
                    id = "SystemStat";
                    layout = "bottom";
                    roundedCorners = true;
                    scale = 1;
                    showBackground = true;
                    statType = "Disk";
                    x = 120;
                    y = 600;
                  }
                  {
                    diskPath = "/";
                    id = "SystemStat";
                    layout = "bottom";
                    roundedCorners = true;
                    showBackground = true;
                    statType = "Disk";
                    x = 120;
                    y = 760;
                  }
                  {
                    diskPath = "/";
                    id = "SystemStat";
                    layout = "bottom";
                    roundedCorners = true;
                    showBackground = true;
                    statType = "GPU";
                    x = 400;
                    y = 120;
                  }
                ];
              }
            ];
          };
        };
      };
    };
}
