{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.programs.webapps;

  mkTauriWrapper =
    {
      name,
      url,
      icon ? null,
      categories ? [ "Network" ],
    }:
    pkgs.rustPlatform.buildRustPackage {
      pname = "tauri-${name}";
      version = "1.0.0";

      # Generate source tree with Cargo.toml, main.rs, and tauri.conf.json
      src = pkgs.runCommand "src-${name}" { } ''
        mkdir -p $out/src-tauri/src
        # Cargo.toml
        cat > $out/src-tauri/Cargo.toml <<EOF
        [package]
        name = "${name}"
        version = "0.1.0"
        edition = "2021"

        [dependencies]
        tauri = "=2.8.5"

        [[bin]]
        name = "${name}"
        path = "src/main.rs"
        EOF

        # main.rs
        cat > $out/src-tauri/src/main.rs <<EOF
        fn main() {
          tauri::Builder::default()
            .run(tauri::generate_context!())
            .expect("error while running ${name}");
        }
        EOF

        # tauri.conf.json
        cat > $out/src-tauri/tauri.conf.json <<EOF
        {
          "package": { "productName": "${name}" },
          "tauri": {
            "windows": [
              { "title": "${name}", "url": "${url}", "width": 1200, "height": 800 }
            ],
            "allowlist": { "all": false },
            "security": {
              "csp": "default-src 'self'; script-src 'none'; object-src 'none'"
            }
          }
        }
        EOF
      '';

      # vendored deps
      cargoLock = {
        lockFile = ./Cargo.lock; # must be generated with `cargo generate-lockfile`
      };

      nativeBuildInputs = [ pkgs.pkg-config ];
      buildInputs = [
        pkgs.webkitgtk
        pkgs.openssl
      ];

      installPhase = ''
        mkdir -p $out/bin
        cp target/release/${name} $out/bin/

        mkdir -p $out/share/applications
        cat > $out/share/applications/${name}.desktop <<EOF
        [Desktop Entry]
        Name=${name}
        Exec=$out/bin/${name}
        Type=Application
        Categories=${lib.concatStringsSep ";" categories};
        ${lib.optionalString (icon != null) "Icon=${icon}"}
        EOF
      '';
    };

in
{
  options.programs.webapps = {
    enable = lib.mkEnableOption "Generate lightweight Tauri-based webapp wrappers";

    apps = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.submodule {
          options = {
            url = lib.mkOption {
              type = lib.types.str;
              description = "The URL to open in the webview.";
            };
            icon = lib.mkOption {
              type = lib.types.nullOr lib.types.path;
              default = null;
              description = "Optional icon for the .desktop file.";
            };
            categories = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ "Network" ];
              description = "List of categories for the .desktop entry.";
            };
          };
        }
      );
      default = { };
      description = "Set of webapps to generate.";
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = lib.attrsets.mapAttrsToList (n: v: mkTauriWrapper ({ name = n; } // v)) cfg.apps;
  };
}
