{
  pkgs,
  lib,
  inputs,
  var,
  ...
}:
{
  catppuccin.vscode = {
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
  programs.vscode = {
    enable = true;
    package = pkgs.vscodium;
    profiles.default = {
      enableExtensionUpdateCheck = false;
      enableUpdateCheck = false;
      extensions = with pkgs.vscode-extensions; [
        esbenp.prettier-vscode
        redhat.vscode-yaml
        signageos.signageos-vscode-sops
        golang.go
        rust-lang.rust-analyzer
        jnoortheen.nix-ide
        nefrob.vscode-just-syntax
        docker.docker
        github.vscode-github-actions
        github.copilot
        github.copilot-chat
        gruntfuggly.todo-tree
        timonwong.shellcheck
        # anthropic.claude-code
        tamasfe.even-better-toml
        mads-hartmann.bash-ide-vscode
        bmalehorn.vscode-fish
        waderyan.gitblame
        alefragnani.project-manager
        wakatime.vscode-wakatime
      ];
      userSettings = {
        # Telemetry and updates
        telemetry.telemetryLevel = "off";
        update.mode = "none";
        extensions.autoUpdate = false;
        redhat.telemetry.enabled = false;

        # Window settings
        window.titleBarStyle = "custom";
        window.density.editorTabHeight = "default";
        # Workbench settings
        workbench = {
          startupEditor = "none";
          workbench.list.smoothScrolling = true;
          editor = {
            empty.hint = "hidden";
            autoLockGroups."mainThreadWebview-markdown.preview" = true;
          };
        };

        # Breadcrumbs
        breadcrumbs.enabled = true;

        # Editor settings
        editor = {
          fontLigatures = true;
          minimap.enabled = false;
          fontFamily = "'${var.fonts.mono}',monospace";
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

        # Search settings
        search.exclude = {
          "**/.direnv" = true;
          "**/.git" = true;
          "**/node_modules" = true;
          "*.lock" = true;
          dist = true;
          tmp = true;
        };

        # Terminal settings
        terminal.integrated = {
          copyOnSelection = true;
          cursorBlinking = true;
          enablePersistentSessions = false;
          hideOnStartup = "whenEmpty";
        };

        # Git settings
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

        # GitHub settings
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

        # Explorer settings
        explorer = {
          confirmDelete = false;
          confirmDragAndDrop = false;
        };

        # SCM settings
        scm = {
          alwaysShowActions = true;
          defaultViewMode = "tree";
        };

        # Files settings
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

        # Prettier settings
        prettier = {
          tabWidth = 2;
          singleAttributePerLine = true;
          bracketSameLine = true;
        };

        # Security settings
        security.workspace.trust.untrustedFiles = "open";

        # Settings sync
        settingsSync = {
          ignoredSettings = [ ];
          ignoredExtensions = [ ];
        };

        # Diff editor
        diffEditor = {
          ignoreTrimWhitespace = false;
          hideUnchangedRegions.enabled = true;
        };

        # Remote settings
        remote.autoForwardPortsSource = "hybrid";

        # Cron settings
        cron-explained = {
          cronstrueOptions.verbose = false;
          codeLens.showTranscript = false;
        };

        # Chat settings
        chat = {
          editing.confirmEditRequestRemoval = false;
          commandCenter.enabled = true;
        };

        # GitHub Copilot settings
        github.copilot = {
          editor.enableAutoCompletions = false;
          enable."*" = false;
        };

        # Claude Code settings
        claudeCode = {
          useTerminal = false;
          enableAutocompletions = true;
          enableInlineEdits = true;
          allowDangerouslySkipPermissions = true;
        };

        # Gitblame settings
        gitblame = {
          ignoreWhitespace = true;
          inlineMessageEnabled = false;
          statusBarMessageEnabled = true;
        };

        # Language-specific: Nix
        nix = {
          enableLanguageServer = true;
          serverPath = "nixd";
          serverSettings.nixd = {
            formatting.command = [ "${lib.getExe pkgs.nixfmt-rfc-style}" ];
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
            diagnostic.suppress = [
              "sema-extra-with"
            ];
          };
          hiddenLanguageServerErrors = [
            "textDocument/definition"
            "unknown node type for definition"
          ];
        };
        "[nix]".editor.defaultFormatter = "jnoortheen.nix-ide";

        # Language-specific: Rust
        rust-analyzer.server.path = "rust-analyzer";

        # Language-specific: Go
        go = {
          toolsManagement.autoUpdate = true;
          inlayHints.assignVariableTypes = true;
        };
        gopls."ui.documentation.hoverKind" = "FullDocumentation";
        "[go]".editor.defaultFormatter = "golang.go";

        # Language-specific: YAML
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

        # Language-specific: JSON
        "[json]".editor.defaultFormatter = "vscode.json-language-features";
        "[jsonc]" = {
          editor = {
            quickSuggestions.strings = true;
            suggest.insertMode = "replace";
          };
        };

        # Language-specific: Fish
        "[fish]".editor.defaultFormatter = "bmalehorn.vscode-fish";

        # Language-specific: Shell
        "[shellscript]".editor.defaultFormatter = "mads-hartmann.bash-ide-vscode";

        # Language-specific: OpenTofu
        opentofu = {
          codelens.referenceCount = true;
          experimentalFeatures.prefillRequiredFields = true;
        };
        "[opentofu]".editor.defaultFormatter = "opentofu.vscode-opentofu";

        # Language-specific: Docker
        "[dockerbake]".editor.defaultFormatter = "docker.docker";
        "[dockercompose]" = {
          editor = {
            insertSpaces = true;
            tabSize = 2;
            autoIndent = "advanced";
            defaultFormatter = "redhat.vscode-yaml";
          };
        };

        # Language-specific: GitHub Actions
        "[github-actions-workflow]".editor.defaultFormatter = "redhat.vscode-yaml";

        # Todo-tree settings
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

        # Project Manager settings
        projectManager = {
          git.baseFolders = [
            "~/"
          ];
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

        # SOPS settings
        sops = {
          configPath = "./.sops.yaml";
          creationEnabled = false;
          defaults = {
            ageKeyFile = "~/.config/sops/age/keys.txt";
          };
        };
      };
    };
  };
  home.file.".vscode-oss/argv.json".text = builtins.toJSON {
    password-store = "gnome-libsecret";
    enable-crash-reporter = false;
    #crash-reporter-id = "38bde5df-002a-4f24-8170-ad11452b15a4";
  };
}
