{ pkgs, lib, ... }:
{

  catppuccin.vscode.profiles.default.enable = true;
  programs.vscode = {
    enable = true;
    package = pkgs.vscodium-fhs;
    profiles.default.extensions = with pkgs.vscode-extensions; [
      esbenp.prettier-vscode
      redhat.vscode-yaml
      signageos.signageos-vscode-sops
      golang.go
      rust-lang.rust-analyzer
      jnoortheen.nix-ide
      nefrob.vscode-just-syntax
      b4dm4n.vscode-nixpkgs-fmt
      docker.docker
      github.vscode-github-actions
      gruntfuggly.todo-tree
      timonwong.shellcheck
      anthropic.claude-code
      tamasfe.even-better-toml
      mads-hartmann.bash-ide-vscode
      bmalehorn.vscode-fish

      waderyan.gitblame
    ];
    profiles.default.userSettings = {
      # Telemetry and updates
      "telemetry.telemetryLevel" = "off";
      "update.mode" = "none";
      "extensions.autoUpdate" = false;
      "redhat.telemetry.enabled" = false;

      # Window settings
      "window.titleBarStyle" = "custom";

      # Workbench settings
      "workbench.startupEditor" = "none";
      "workbench.editor.showTabs" = "single";
      "workbench.editor.empty.hint" = "hidden";
      "workbench.editor.autoLockGroups" = {
        "mainThreadWebview-markdown.preview" = true;
      };

      # Editor settings
      "editor.fontLigatures" = true;
      "editor.minimap.enabled" = false;
      "editor.fontFamily" =
        "'JetBrainsMono Nerd Font','CaskaydiaCove NFM','Cascadia Code', 'Fira Code Medium', monospace";
      "editor.defaultFormatter" = "esbenp.prettier-vscode";
      "editor.formatOnPaste" = true;
      "editor.formatOnSave" = true;
      "editor.linkedEditing" = true;
      "editor.tabCompletion" = "on";
      "editor.cursorSmoothCaretAnimation" = "on";
      "editor.renderControlCharacters" = false;
      "editor.smoothScrolling" = true;
      "editor.cursorStyle" = "block";
      "editor.cursorBlinking" = "phase";
      "editor.find.cursorMoveOnType" = true;
      "editor.suggest.preview" = true;
      "editor.fontSize" = 15;
      "editor.tabSize" = 2;
      "editor.accessibilitySupport" = "off";
      "editor.bracketPairColorization.independentColorPoolPerBracketType" = true;
      "editor.renderWhitespace" = "none";

      # Search settings
      "search.exclude" = {
        "**/.direnv" = true;
        "**/.git" = true;
        "**/node_modules" = true;
        "*.lock" = true;
        "dist" = true;
        "tmp" = true;
      };

      # Terminal settings
      "terminal.integrated.env.linux" = { };
      "terminal.integrated.copyOnSelection" = true;
      "terminal.integrated.cursorBlinking" = true;
      "terminal.integrated.enablePersistentSessions" = false;
      "terminal.integrated.hideOnStartup" = "whenEmpty";

      # Git settings
      "git.autofetch" = true;
      "git.enableSmartCommit" = true;
      "git.confirmSync" = false;
      "git.autoStash" = true;
      "git.closeDiffOnOperation" = true;
      "git.fetchOnPull" = true;
      "git.mergeEditor" = true;
      "git.pruneOnFetch" = true;
      "git.pullBeforeCheckout" = true;
      "git.rebaseWhenSync" = true;
      "git.ignoreRebaseWarning" = true;

      # GitHub settings
      "github.gitProtocol" = "ssh";
      "githubPullRequests.fileListLayout" = "flat";
      "githubPullRequests.pullBranch" = "never";
      "githubIssues.queries" = [
        {
          "label" = "My Issues";
          "query" = "default";
        }
        {
          "label" = "Created Issues";
          "query" = "author:\${user} state:open repo:\${owner}/\${repository} sort:created-desc";
        }
        {
          "label" = "Recent Issues";
          "query" = "state:open repo:\${owner}/\${repository} sort:updated-desc";
        }
      ];

      # Explorer settings
      "explorer.confirmDelete" = false;
      "explorer.confirmDragAndDrop" = false;

      # SCM settings
      "scm.alwaysShowActions" = true;
      "scm.defaultViewMode" = "tree";

      # Files settings
      "files.associations" = {
        "*.tf" = "opentofu";
        "CODEOWNERS" = "plaintext";
      };
      "files.exclude" = {
        "**/.trunk/*actions/" = true;
        "**/.trunk/*logs/" = true;
        "**/.trunk/*notifications/" = true;
        "**/.trunk/*out/" = true;
        "**/.trunk/*plugins/" = true;
      };
      "files.watcherExclude" = {
        "**/.trunk/*actions/" = true;
        "**/.trunk/*logs/" = true;
        "**/.trunk/*notifications/" = true;
        "**/.trunk/*out/" = true;
        "**/.trunk/*plugins/" = true;
      };

      # Prettier settings
      "prettier.tabWidth" = 2;
      "prettier.singleAttributePerLine" = true;
      "prettier.bracketSameLine" = true;

      # Security settings
      "security.workspace.trust.untrustedFiles" = "open";

      # Settings sync
      "settingsSync.ignoredSettings" = [ ];
      "settingsSync.ignoredExtensions" = [ ];

      # Diff editor
      "diffEditor.ignoreTrimWhitespace" = false;
      "diffEditor.hideUnchangedRegions.enabled" = true;

      # Remote settings
      "remote.autoForwardPortsSource" = "hybrid";

      # Catppuccin settings
      "catppuccin.accentColor" = "blue";

      # Cron settings
      "cron-explained.cronstrueOptions.verbose" = false;
      "cron-explained.codeLens.showTranscript" = false;

      # Chat settings
      "chat.editing.confirmEditRequestRemoval" = false;

      # Gitblame settings
      "gitblame.ignoreWhitespace" = true;
      "gitblame.inlineMessageEnabled" = true;

      # Language-specific: Nix
      "nix.enableLanguageServer" = true;
      "nix.serverPath" = "nixd";
      "nix.formatterPath" = "nixfmt";
      "nix.serverSettings" = {
        # settings for 'nixd' LSP
        "nixd" = {
          "nixpkgs" = {
            "expr" =
              "import (builtins.getFlake /nix/store/hbw0plb1r3bd6hngmsmcy9rpz0xg2hj8-source).inputs.nixpkgs { }";
          };
          "formatting" = {
            "command" = [ "nixfmt" ];
          };
          "options" = {
            "enable" = true;
            "nixos" = {
              "expr" =
                "(builtins.getFlake /nix/store/hbw0plb1r3bd6hngmsmcy9rpz0xg2hj8-source).nixosConfigurations.thor.options";
            };
            "home-manager" = {
              "expr" =
                "(builtins.getFlake /nix/store/hbw0plb1r3bd6hngmsmcy9rpz0xg2hj8-source).nixosConfigurations.thor.options.home-manager.users.type.getSubOptions []";
            };
            "flake-parts" = {
              "expr" = "(builtins.getFlake(builtins.toString ./.)).debug.options";
            };
            "flake-parts2" = {
              "expr" = "(builtins.getFlake(builtins.toString ./.)).currentSystem.options";
            };
          };
        };
      };
      "nixpkgs-fmt.path" = "${lib.getExe pkgs.nixfmt-rfc-style}";
      "[nix]" = {
        "editor.defaultFormatter" = "B4dM4n.nixpkgs-fmt";
      };

      # Language-specific: Rust
      "rust-analyzer.server.path" = "rust-analyzer";

      # Language-specific: Go
      "go.toolsManagement.autoUpdate" = true;
      "go.inlayHints.assignVariableTypes" = true;
      "gopls" = {
        "ui.documentation.hoverKind" = "FullDocumentation";
      };
      "[go]" = {
        "editor.defaultFormatter" = "golang.go";
      };

      # Language-specific: YAML
      "yaml.format.enable" = true;
      "yaml.validate" = true;
      "[yaml]" = {
        "editor.defaultFormatter" = "redhat.vscode-yaml";
        "editor.autoIndent" = "full";
        "editor.detectIndentation" = true;
        "diffEditor.ignoreTrimWhitespace" = true;
      };

      # Language-specific: JSON
      "[json]" = {
        "editor.defaultFormatter" = "vscode.json-language-features";
      };
      "[jsonc]" = {
        "editor.quickSuggestions" = {
          "strings" = true;
        };
        "editor.suggest.insertMode" = "replace";
      };

      # Language-specific: Fish
      "[fish]" = {
        "editor.defaultFormatter" = "bmalehorn.vscode-fish";
      };

      # Language-specific: Shell
      "[shellscript]" = {
        "editor.defaultFormatter" = "mads-hartmann.bash-ide-vscode";
      };

      # Language-specific: OpenTofu
      "opentofu.codelens.referenceCount" = true;
      "opentofu.experimentalFeatures.prefillRequiredFields" = true;
      "[opentofu]" = {
        "editor.defaultFormatter" = "opentofu.vscode-opentofu";
      };

      # Language-specific: Docker
      "[dockerbake]" = {
        "editor.defaultFormatter" = "docker.docker";
      };
      "[dockercompose]" = {
        "editor.insertSpaces" = true;
        "editor.tabSize" = 2;
        "editor.autoIndent" = "advanced";
        "editor.defaultFormatter" = "redhat.vscode-yaml";
      };

      # Language-specific: GitHub Actions
      "[github-actions-workflow]" = {
        "editor.defaultFormatter" = "redhat.vscode-yaml";
      };

      # Todo-tree settings
      "todo-tree.general.showActivityBarBadge" = true;
      "todo-tree.filtering.ignoreGitSubmodules" = true;
      "todo-tree.tree.showCountsInTree" = true;
      "todo-tree.tree.buttons.scanMode" = true;
      "todo-tree.filtering.useBuiltInExcludes" = "file and search excludes";

      # SOPS settings
      "sops.configPath" = "./.sops.yaml";
      "sops.creationEnabled" = true;
    };
  };
  home.file.".vscode-oss/argv.json".text = builtins.toJSON {
    password-store = "gnome-libsecret";
    enable-crash-reporter = false;
    crash-reporter-id = "38bde5df-002a-4f24-8170-ad11452b15a4";
  };
}
