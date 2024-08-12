{ ... }:
{
  modules.desktop.vesktop = {
    enable = true;
    settings = {
      minimizeToTray = true;
      discordBranch = "stable";
      arRPC = true;
      splashColor = "oklab(0.921539 -0.00903007 -0.00387452)";
      splashBackground = "oklab(0.254186 -0.0027893 -0.0239889)";
    };
    extraConfig = {
      autoUpdate = false;
      autoUpdateNotification = true;
      cloud = {
        authenticated = false;
        settingsSync = false;
        settingsSyncVersion = 1721466192851;
        url = "https://api.vencord.dev/";
      };
      disableMinSize = false;
      enableReactDevtools = true;
      enabledThemes = [ ];
      frameless = true;
      # notifications = {
      #   logLimit = 50;
      #   position = "bottom-right";
      #   timeout = 5000;
      #   useNative = "not-focused";
      # };
      notifyAboutUpdates = true;
      plugins = {
        AlwaysAnimate = {
          enabled = false;
        };
        AlwaysTrust = {
          enabled = false;
        };
        AnonymiseFileNames = {
          enabled = false;
        };
        AppleMusicRichPresence = {
          enabled = false;
        };
        AutomodContext = {
          enabled = false;
        };
        BANger = {
          enabled = false;
        };
        BadgeAPI = {
          enabled = true;
        };
        BetterFolders = {
          enabled = false;
        };
        BetterGifAltText = {
          enabled = false;
        };
        BetterGifPicker = {
          enabled = true;
        };
        BetterNotesBox = {
          enabled = false;
        };
        BetterRoleContext = {
          enabled = false;
        };
        BetterRoleDot = {
          enabled = false;
        };
        BetterSessions = {
          enabled = false;
        };
        BetterSettings = {
          enabled = false;
        };
        BetterUploadButton = {
          enabled = false;
        };
        BiggerStreamPreview = {
          enabled = false;
        };
        BlurNSFW = {
          enabled = false;
        };
        CallTimer = {
          enabled = false;
        };
        ChatInputButtonAPI = {
          enabled = true;
        };
        ClearURLs = {
          enabled = false;
        };
        ClientTheme = {
          enabled = false;
        };
        ColorSighted = {
          enabled = false;
        };
        CommandsAPI = {
          enabled = true;
        };
        ConsoleJanitor = {
          enabled = false;
        };
        ConsoleShortcuts = {
          enabled = false;
        };
        ContextMenuAPI = {
          enabled = true;
        };
        CopyEmojiMarkdown = {
          enabled = false;
        };
        CopyUserURLs = {
          enabled = false;
        };
        CrashHandler = {
          enabled = true;
        };
        CtrlEnterSend = {
          enabled = false;
        };
        CustomIdle = {
          enabled = false;
        };
        CustomRPC = {
          enabled = false;
        };
        Dearrow = {
          enabled = false;
        };
        Decor = {
          enabled = true;
          agreedToGuidelines = false;
        };
        DisableCallIdle = {
          enabled = false;
        };
        DontRoundMyTimestamps = {
          enabled = false;
        };
        EmoteCloner = {
          enabled = false;
        };
        Experiments = {
          enabled = false;
        };
        F8Break = {
          enabled = false;
        };
        FakeNitro = {
          enabled = true;
          disableEmbedPermissionCheck = false;
          emojiSize = 48;
          enableEmojiBypass = true;
          enableStickerBypass = true;
          enableStreamQualityBypass = true;
          hyperLinkText = "{{NAME}}";
          transformCompoundSentence = false;
          transformEmojis = true;
          transformStickers = true;
          useHyperLinks = true;
        };
        FakeProfileThemes = {
          enabled = true;
          nitroFirst = true;
        };
        FavoriteEmojiFirst = {
          enabled = false;
        };
        FavoriteGifSearch = {
          enabled = false;
        };
        FixCodeblockGap = {
          enabled = false;
        };
        FixSpotifyEmbeds = {
          enabled = true;
        };
        FixYoutubeEmbeds = {
          enabled = true;
        };
        ForceOwnerCrown = {
          enabled = false;
        };
        FriendInvites = {
          enabled = false;
        };
        FriendsSince = {
          enabled = true;
        };
        GameActivityToggle = {
          enabled = false;
        };
        GifPaste = {
          enabled = false;
        };
        GreetStickerPicker = {
          enabled = false;
        };
        HideAttachments = {
          enabled = false;
        };
        IgnoreActivities = {
          enabled = false;
        };
        ImageLink = {
          enabled = false;
        };
        ImageZoom = {
          enabled = true;
        };
        ImplicitRelationships = {
          enabled = false;
        };
        InvisibleChat = {
          enabled = false;
        };
        KeepCurrentChannel = {
          enabled = false;
        };
        LastFMRichPresence = {
          enabled = false;
        };
        LoadingQuotes = {
          enabled = false;
        };
        MaskedLinkPaste = {
          enabled = false;
        };
        MemberCount = {
          enabled = false;
        };
        MemberListDecoratorsAPI = {
          enabled = false;
        };
        MentionAvatars = {
          enabled = false;
        };
        MessageAccessoriesAPI = {
          enabled = true;
        };
        MessageClickActions = {
          enabled = false;
        };
        MessageDecorationsAPI = {
          enabled = false;
        };
        MessageEventsAPI = {
          enabled = true;
        };
        MessageLatency = {
          enabled = false;
        };
        MessageLinkEmbeds = {
          enabled = false;
        };
        MessageLogger = {
          enabled = false;
        };
        MessagePopoverAPI = {
          enabled = true;
        };
        MessageTags = {
          enabled = false;
        };
        MessageUpdaterAPI = {
          enabled = false;
        };
        MoreCommands = {
          enabled = false;
        };
        MoreKaomoji = {
          enabled = false;
        };
        MoreUserTags = {
          enabled = false;
        };
        Moyai = {
          enabled = false;
          ignoreBlocked = true;
          ignoreBots = true;
          quality = "Normal";
          triggerWhenUnfocused = true;
          volume = 0.5;
        };
        MutualGroupDMs = {
          enabled = false;
        };
        NSFWGateBypass = {
          enabled = true;
        };
        NewGuildSettings = {
          enabled = false;
        };
        NoBlockedMessages = {
          enabled = false;
        };
        NoDefaultHangStatus = {
          enabled = false;
        };
        NoDevtoolsWarning = {
          enabled = false;
        };
        NoF1 = {
          enabled = false;
        };
        NoMosaic = {
          enabled = false;
        };
        NoOnboardingDelay = {
          enabled = false;
        };
        NoPendingCount = {
          enabled = false;
        };
        NoProfileThemes = {
          enabled = false;
        };
        NoReplyMention = {
          enabled = false;
        };
        NoScreensharePreview = {
          enabled = false;
        };
        NoServerEmojis = {
          enabled = false;
        };
        NoTrack = {
          enabled = true;
          disableAnalytics = true;
        };
        NoTypingAnimation = {
          enabled = false;
        };
        NoUnblockToJump = {
          enabled = false;
        };
        NormalizeMessageLinks = {
          enabled = false;
        };
        NoticesAPI = {
          enabled = true;
        };
        NotificationVolume = {
          enabled = false;
        };
        OnePingPerDM = {
          enabled = false;
        };
        OpenInApp = {
          enabled = false;
        };
        OverrideForumDefaults = {
          enabled = false;
        };
        PartyMode = {
          enabled = false;
        };
        PauseInvitesForever = {
          enabled = false;
        };
        PermissionFreeWill = {
          enabled = false;
        };
        PermissionsViewer = {
          enabled = false;
        };
        PictureInPicture = {
          enabled = false;
        };
        PinDMs = {
          enabled = false;
        };
        PlainFolderIcon = {
          enabled = false;
        };
        PlatformIndicators = {
          enabled = false;
        };
        PreviewMessage = {
          enabled = false;
        };
        PronounDB = {
          enabled = false;
        };
        QuickMention = {
          enabled = false;
        };
        QuickReply = {
          enabled = false;
        };
        ReactErrorDecoder = {
          enabled = false;
        };
        ReadAllNotificationsButton = {
          enabled = false;
        };
        RelationshipNotifier = {
          enabled = false;
        };
        ReplaceGoogleSearch = {
          enabled = false;
        };
        ReplyTimestamp = {
          enabled = false;
        };
        RevealAllSpoilers = {
          enabled = false;
        };
        ReverseImageSearch = {
          enabled = false;
        };
        ReviewDB = {
          enabled = false;
          hideBlockedUsers = true;
          hideTimestamps = false;
          notifyReviews = true;
          showWarning = true;
        };
        RoleColorEverywhere = {
          enabled = false;
        };
        SearchReply = {
          enabled = false;
        };
        SecretRingToneEnabler = {
          enabled = false;
        };
        SendTimestamps = {
          enabled = false;
        };
        ServerInfo = {
          enabled = false;
        };
        ServerListAPI = {
          enabled = false;
        };
        ServerListIndicators = {
          enabled = false;
        };
        Settings = {
          enabled = true;
          settingsLocation = "aboveNitro";
        };
        ShikiCodeblocks = {
          enabled = false;
        };
        ShowAllMessageButtons = {
          enabled = false;
        };
        ShowAllRoles = {
          enabled = false;
        };
        ShowConnections = {
          enabled = false;
        };
        ShowHiddenChannels = {
          enabled = false;
        };
        ShowHiddenThings = {
          enabled = false;
        };
        ShowMeYourName = {
          enabled = false;
        };
        ShowTimeoutDuration = {
          enabled = false;
        };
        SilentMessageToggle = {
          enabled = false;
        };
        SilentTyping = {
          enabled = false;
        };
        SortFriendRequests = {
          enabled = false;
        };
        SpotifyControls = {
          enabled = false;
        };
        SpotifyCrack = {
          enabled = false;
        };
        SpotifyShareCommands = {
          enabled = false;
        };
        StartupTimings = {
          enabled = false;
        };
        StreamerModeOnStream = {
          enabled = false;
        };
        Summaries = {
          enabled = false;
        };
        SuperReactionTweaks = {
          enabled = false;
        };
        SupportHelper = {
          enabled = false;
        };
        TextReplace = {
          enabled = false;
        };
        ThemeAttributes = {
          enabled = false;
        };
        TimeBarAllActivities = {
          enabled = false;
        };
        Translate = {
          enabled = false;
          autoTranslate = false;
          receivedInput = "auto";
          receivedOutput = "en";
          sentInput = "auto";
          sentOutput = "en";
          showChatBarButton = true;
        };
        TypingIndicator = {
          enabled = false;
        };
        TypingTweaks = {
          enabled = false;
        };
        USRBG = {
          enabled = true;
          nitroFirst = false;
          voiceBackground = true;
        };
        Unindent = {
          enabled = false;
        };
        UnlockedAvatarZoom = {
          enabled = false;
        };
        UnsuppressEmbeds = {
          enabled = false;
        };
        UrbanDictionary = {
          enabled = false;
        };
        UserSettingsAPI = {
          enabled = true;
        };
        UserVoiceShow = {
          enabled = false;
        };
        ValidReply = {
          enabled = false;
        };
        ValidUser = {
          enabled = false;
        };
        VcNarrator = {
          enabled = false;
        };
        VencordToolbox = {
          enabled = false;
        };
        ViewIcons = {
          enabled = false;
        };
        ViewRaw = {
          enabled = false;
        };
        VoiceChatDoubleClick = {
          enabled = false;
        };
        VoiceDownload = {
          enabled = false;
        };
        VoiceMessages = {
          enabled = false;
        };
        WatchTogetherAdblock = {
          enabled = false;
        };
        WebContextMenus = {
          addBack = true;
          enabled = true;
        };
        WebKeybinds = {
          enabled = true;
        };
        "WebRichPresence (arRPC)" = {
          enabled = false;
        };
        WebScreenShareFixes = {
          enabled = true;
        };
        WhoReacted = {
          enabled = false;
        };
        Wikisearch = {
          enabled = false;
        };
        XSOverlay = {
          enabled = false;
        };
        iLoveSpam = {
          enabled = false;
        };
        oneko = {
          enabled = false;
        };
        petpet = {
          enabled = false;
        };
      };
      themeLinks = [
        "https://capnkitten.github.io/BetterDiscord/Themes/Material-Discord/css/source.css"
      ];
      transparent = false;
      useQuickCss = true;
      winCtrlQ = false;
      winNativeTitleBar = false;
    };
  };
}
