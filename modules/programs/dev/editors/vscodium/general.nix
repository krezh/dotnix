{
  flake.modules.homeManager.editors =
    {
      pkgs,
      config,
      ...
    }:
    {
      vscodium.extensionIds = [
        "esbenp.prettier-vscode"
        "signageos.signageos-vscode-sops"
        "gruntfuggly.todo-tree"
        "waderyan.gitblame"
        "alefragnani.project-manager"
        "mkhl.direnv"
        "blueglassblock.better-json5"
        "editorconfig.editorconfig"
        "usernamehw.errorlens"
        "oderwat.indent-rainbow"
        "mhutchie.git-graph"
        "anthropic.claude-code"
        "zizmor.zizmor-vscode"
        "christian-kohler.path-intellisense"
        "gitHub.vscode-pull-request-github"
      ];

      catppuccin.vscodium = {
        profiles.default = {
          accent = "blue";
          settings = {
            boldKeywords = true;
            italicComments = true;
            italicKeywords = true;
            colorOverrides = { };
            customUIColors = { };
            workbenchMode = "minimal";
            bracketMode = "rainbow";
            extraBordersEnabled = false;
          };
        };
      };

      programs.vscodium = {
        enable = true;
        mutableExtensionsDir = true;
        profiles.default = {
          enableExtensionUpdateCheck = false;
          enableUpdateCheck = false;
          extensions = pkgs.nix4vscode.forVscodeVersion config.programs.vscodium.package.version config.vscodium.extensionIds;
          userSettings = {
            telemetry.telemetryLevel = "off";
            update.mode = "none";
            extensions.autoUpdate = false;
            redhat.telemetry.enabled = false;
            window.titleBarStyle = "custom";
            window.density.editorTabHeight = "default";
            workbench = {
              startupEditor = "none";
              list.smoothScrolling = true;
              editor = {
                empty.hint = "hidden";
                autoLockGroups."mainThreadWebview-markdown.preview" = true;
              };
              editorAssociations = {
                "*.qrc" = "qt-core.qrcEditor";
                "{git,gitlens,chat-editing-snapshot-text-model,git-graph,git-graph-3}:/**/*.qrc" = "default";
                "{git,gitlens,chat-editing-snapshot-text-model,git-graph,git-graph-3}:/**/*.ui" = "default";
              };
            };
            breadcrumbs.enabled = true;
            editor = {
              fontLigatures = true;
              minimap.enabled = false;
              fontFamily = "'${config.var.fonts.mono}',monospace";
              defaultFormatter = "esbenp.prettier-vscode";
              formatOnPaste = true;
              formatOnSave = true;
              linkedEditing = true;
              tabCompletion = "on";
              cursorSmoothCaretAnimation = "on";
              renderControlCharacters = false;
              smoothScrolling = true;
              cursorStyle = "block";
              cursorBlinking = "phase";
              find.cursorMoveOnType = true;
              suggest.preview = true;
              fontSize = 16;
              tabSize = 2;
              accessibilitySupport = "off";
              bracketPairColorization.independentColorPoolPerBracketType = true;
              renderWhitespace = "none";
              inlayHints.enabled = "on";
              stickyScroll.enabled = true;
              selectionClipboard = false;
              autoIndentOnPaste = true;
              guides = {
                bracketPairs = true;
                bracketPairsHorizontal = true;
                highlightActiveBracketPair = true;
              };
            };
            search.exclude = {
              "**/.direnv" = true;
              "**/.git" = true;
              "**/node_modules" = true;
              "*.lock" = true;
              dist = true;
              tmp = true;
            };
            terminal.integrated = {
              copyOnSelection = true;
              cursorBlinking = true;
              enablePersistentSessions = false;
              hideOnStartup = "whenEmpty";
            };
            git = {
              autofetch = true;
              enableSmartCommit = true;
              confirmSync = false;
              autoStash = true;
              closeDiffOnOperation = true;
              fetchOnPull = true;
              mergeEditor = true;
              pruneOnFetch = true;
              pullBeforeCheckout = true;
              rebaseWhenSync = true;
              ignoreRebaseWarning = true;
              blame = {
                statusBarItem.enabled = true;
                editorDecoration.enabled = true;
              };
            };
            github.gitProtocol = "ssh";
            githubPullRequests = {
              fileListLayout = "flat";
              pullBranch = "never";
            };
            githubIssues.queries = [
              {
                label = "My Issues";
                query = "default";
              }
              {
                label = "Created Issues";
                query = "author:\${user} state:open repo:\${owner}/\${repository} sort:created-desc";
              }
              {
                label = "Recent Issues";
                query = "state:open repo:\${owner}/\${repository} sort:updated-desc";
              }
            ];
            explorer = {
              confirmDelete = false;
              confirmDragAndDrop = false;
              autoReveal = true;
            };
            scm = {
              alwaysShowActions = true;
              defaultViewMode = "tree";
            };
            files = {
              trimTrailingWhitespace = true;
              associations = {
                "*.tf" = "opentofu";
                CODEOWNERS = "plaintext";
              };
              exclude = {
                "**/.trunk/*actions/" = true;
                "**/.trunk/*logs/" = true;
                "**/.trunk/*notifications/" = true;
                "**/.trunk/*out/" = true;
                "**/.trunk/*plugins/" = true;
              };
              watcherExclude = {
                "**/.trunk/*actions/" = true;
                "**/.trunk/*logs/" = true;
                "**/.trunk/*notifications/" = true;
                "**/.trunk/*out/" = true;
                "**/.trunk/*plugins/" = true;
              };
            };
            prettier = {
              tabWidth = 2;
              singleAttributePerLine = true;
              bracketSameLine = true;
            };
            security.workspace.trust.untrustedFiles = "open";
            settingsSync = {
              ignoredSettings = [ ];
              ignoredExtensions = [ ];
            };
            diffEditor = {
              ignoreTrimWhitespace = false;
              hideUnchangedRegions.enabled = true;
              renderSideBySide = false;
            };
            remote.autoForwardPortsSource = "hybrid";
            cron-explained = {
              cronstrueOptions.verbose = false;
              codeLens.showTranscript = false;
            };
            chat = {
              editing.confirmEditRequestRemoval = false;
              commandCenter.enabled = false;
              disableAIFeatures = true;
            };
            claudeCode = {
              useTerminal = false;
              enableAutocompletions = true;
              enableInlineEdits = true;
              allowDangerouslySkipPermissions = true;
              preferredLocation = "panel";
            };
            gitblame = {
              ignoreWhitespace = true;
              inlineMessageEnabled = false;
              statusBarMessageEnabled = true;
            };
            "[json]".editor.defaultFormatter = "vscode.json-language-features";
            "[jsonc]" = {
              editor = {
                quickSuggestions.strings = true;
                suggest.insertMode = "replace";
              };
            };
            todo-tree = {
              general.showActivityBarBadge = true;
              filtering = {
                ignoreGitSubmodules = true;
                useBuiltInExcludes = "file and search excludes";
              };
              tree = {
                showCountsInTree = true;
                buttons.scanMode = true;
              };
            };
            indentRainbow.indicatorStyle = "light";
            indentRainbow.lightIndicatorStyleLineWidth = 2;
            projectManager = {
              git.baseFolders = [ "~/" ];
              git.ignoredFolders = [
                "node_modules"
                "out"
                "typings"
                "test"
                "fork*"
                ".cache"
              ];
              sortList = "Recent";
              showProjectNameInStatusBar = true;
              openInNewWindowWhenClickingInStatusBar = false;
            };
            sops = {
              configPath = "./.sops.yaml";
              creationEnabled = false;
              defaults.ageKeyFile = "~/.config/sops/age/keys.txt";
            };
          };
        };
      };

      home.file.".vscode-oss/argv.json".text = builtins.toJSON {
        password-store = "gnome-libsecret";
        enable-crash-reporter = false;
      };
    };
}
