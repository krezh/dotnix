{inputs, ...}: {
  flake.modules.homeManager.editors = {
    lib,
    pkgs,
    config,
    ...
  }: {
    catppuccin.vscode = {
      profiles.default = {
        accent = "blue";
        settings = {
          boldKeywords = true;
          italicComments = true;
          italicKeywords = true;
          colorOverrides = {};
          customUIColors = {};
          workbenchMode = "minimal";
          bracketMode = "rainbow";
          extraBordersEnabled = false;
        };
      };
    };

    programs.vscode = {
      enable = true;
      package = pkgs.vscodium;
      mutableExtensionsDir = true;
      profiles.default = {
        enableExtensionUpdateCheck = false;
        enableUpdateCheck = false;
        extensions = (
          pkgs.nix4vscode.forVscodeVersion (config.programs.vscode.package.version) [
            "esbenp.prettier-vscode"
            "redhat.vscode-yaml"
            "signageos.signageos-vscode-sops"
            "golang.go"
            "rust-lang.rust-analyzer"
            "jnoortheen.nix-ide"
            "nefrob.vscode-just-syntax"
            "docker.docker"
            "github.vscode-github-actions"
            "github.copilot-chat"
            "gruntfuggly.todo-tree"
            "timonwong.shellcheck"
            "tamasfe.even-better-toml"
            "mads-hartmann.bash-ide-vscode"
            "bmalehorn.vscode-fish"
            "waderyan.gitblame"
            "alefragnani.project-manager"
            "wakatime.vscode-wakatime"
            "theqtcompany.qt-core"
            "theqtcompany.qt-qml"
            "theqtcompany.qt-ui"
            "mkhl.direnv"
            "opentofu.vscode-opentofu"
            "blueglassblock.better-json5"
            "editorconfig.editorconfig"
            "usernamehw.errorlens"
            "oderwat.indent-rainbow"
            "mhutchie.git-graph"
            "anthropic.claude-code"
            "sst-dev.opencode"
          ]
        );
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
              "*.copilotmd" = "vscode.markdown.preview.editor";
              "*.qrc" = "qt-core.qrcEditor";
              "{git,gitlens,chat-editing-snapshot-text-model,copilot,git-graph,git-graph-3}:/**/*.qrc" = "default";
              "{git,gitlens,chat-editing-snapshot-text-model,copilot,git-graph,git-graph-3}:/**/*.ui" = "default";
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
            autoIndentOnPaste = false;
            guides.bracketPairs = false;
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
          };
          scm = {
            alwaysShowActions = true;
            defaultViewMode = "tree";
          };
          files = {
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
            ignoredSettings = [];
            ignoredExtensions = [];
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
            commandCenter.enabled = true;
            disableAIFeatures = false;
          };
          github.copilot = {
            editor.enableAutoCompletions = false;
            enable."*" = false;
          };
          claudeCode = {
            useTerminal = false;
            enableAutocompletions = true;
            enableInlineEdits = true;
            allowDangerouslySkipPermissions = true;
          };
          gitblame = {
            ignoreWhitespace = true;
            inlineMessageEnabled = false;
            statusBarMessageEnabled = true;
          };
          nix = {
            enableLanguageServer = true;
            serverPath = "nixd";
            serverSettings.nixd = {
              formatting.command = ["${lib.getExe pkgs.alejandra}"];
              nixpkgs.expr = "import ${inputs.nixpkgs} { }";
              options = {
                nixos.expr = ''
                  (let
                    pkgs = import ${inputs.nixpkgs} { };
                  in (pkgs.lib.evalModules {
                    modules = (import ${inputs.nixpkgs}/nixos/modules/module-list.nix) ++ [
                      ({...}: { nixpkgs.hostPlatform = "${pkgs.stdenv.hostPlatform.system}"; })
                    ];
                  })).options
                '';
                home-manager.expr = ''
                  (let
                    pkgs = import ${inputs.nixpkgs} { };
                    lib = import ${inputs.home-manager}/modules/lib/stdlib-extended.nix pkgs.lib;
                  in (lib.evalModules {
                    modules = (import ${inputs.home-manager}/modules/modules.nix) {
                      inherit lib pkgs;
                      check = false;
                    };
                  })).options
                '';
              };
              diagnostic.suppress = ["sema-extra-with"];
            };
            hiddenLanguageServerErrors = [
              "textDocument/definition"
              "unknown node type for definition"
            ];
          };
          "[nix]".editor.defaultFormatter = "jnoortheen.nix-ide";
          rust-analyzer.server.path = "rust-analyzer";
          go = {
            toolsManagement.autoUpdate = true;
            inlayHints.assignVariableTypes = true;
          };
          gopls."ui.documentation.hoverKind" = "FullDocumentation";
          "[go]".editor.defaultFormatter = "golang.go";
          yaml = {
            format.enable = true;
            validate = true;
          };
          "[yaml]" = {
            editor = {
              defaultFormatter = "redhat.vscode-yaml";
              autoIndent = "full";
              detectIndentation = true;
            };
            diffEditor.ignoreTrimWhitespace = true;
          };
          "[json]".editor.defaultFormatter = "vscode.json-language-features";
          "[jsonc]" = {
            editor = {
              quickSuggestions.strings = true;
              suggest.insertMode = "replace";
            };
          };
          "[fish]".editor.defaultFormatter = "bmalehorn.vscode-fish";
          "[shellscript]".editor.defaultFormatter = "mads-hartmann.bash-ide-vscode";
          opentofu = {
            codelens.referenceCount = true;
            experimentalFeatures.prefillRequiredFields = true;
            languageServer = {
              path = lib.getExe pkgs.tofu-ls;
              tofu.path = lib.getExe pkgs.opentofu;
            };
          };
          "[opentofu]".editor.defaultFormatter = "opentofu.vscode-opentofu";
          "[dockerbake]".editor.defaultFormatter = "docker.docker";
          "[dockercompose]" = {
            editor = {
              insertSpaces = true;
              tabSize = 2;
              autoIndent = "advanced";
              defaultFormatter = "redhat.vscode-yaml";
            };
          };
          "[github-actions-workflow]".editor.defaultFormatter = "redhat.vscode-yaml";
          "[qml]".editor.defaultFormatter = "theqtcompany.qt-qml";
          qt-core.additionalQtPaths = [
            {
              name = "Qt6-nix";
              path = "${pkgs.qt6.qtbase}/bin/qtpaths";
            }
          ];
          qt-qml = {
            doNotAskForQmllsDownload = true;
            qmlls = {
              useQmlImportPathEnvVar = true;
              customExePath = "${pkgs.qt6.qtdeclarative}/bin/qmlls";
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
            git.baseFolders = ["~/"];
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
