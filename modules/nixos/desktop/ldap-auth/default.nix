{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.nixosModules.desktop.ldap-auth;
in
{
  options.nixosModules.desktop.ldap-auth = {
    enable = lib.mkEnableOption "LDAP authentication using SSSD";

    server = lib.mkOption {
      type = lib.types.str;
      description = "The hostname of the LDAP server (without protocol)";
      example = "ldap.company.com";
    };

    port = lib.mkOption {
      type = lib.types.int;
      default = 636;
      description = "LDAP server port";
    };

    baseDN = lib.mkOption {
      type = lib.types.str;
      description = "The base distinguished name for LDAP searches";
      example = "dc=company,dc=com";
    };

    domain = lib.mkOption {
      type = lib.types.str;
      description = "SSSD domain name";
      example = "company.com";
    };

    schema = lib.mkOption {
      type = lib.types.enum [
        "rfc2307"
        "rfc2307bis"
        "ipa"
        "ad"
      ];
      default = "rfc2307bis";
      description = "LDAP schema type";
    };

    serviceAccount = {
      username = lib.mkOption {
        type = lib.types.str;
        description = "Service account username for LDAP binding";
        example = "sssd-service";
      };

      passwordFile = lib.mkOption {
        type = lib.types.path;
        description = "Path to file containing the service account password";
        example = "/run/secrets/ldap-service-password";
      };
    };

    userSearch = {
      base = lib.mkOption {
        type = lib.types.str;
        default = "ou=users";
        description = "User search base (relative to baseDN)";
      };

      objectClass = lib.mkOption {
        type = lib.types.str;
        default = "user";
        description = "LDAP object class for users";
      };

      nameAttribute = lib.mkOption {
        type = lib.types.str;
        default = "cn";
        description = "LDAP attribute for username";
      };
    };

    groupSearch = {
      base = lib.mkOption {
        type = lib.types.str;
        default = "";
        description = "Group search base (relative to baseDN, empty means use baseDN)";
      };

      objectClass = lib.mkOption {
        type = lib.types.str;
        default = "group";
        description = "LDAP object class for groups";
      };

      nameAttribute = lib.mkOption {
        type = lib.types.str;
        default = "cn";
        description = "LDAP attribute for group name";
      };
    };

    accessControl = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Whether to enable LDAP-based access control";
      };

      allowedGroup = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Group DN that users must be members of to login";
        example = "cn=authentik Admins,ou=groups,dc=company,dc=com";
      };
    };

    enableSSH = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Whether to enable SSH service in SSSD";
    };

    createHomeDirectory = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Whether to automatically create home directories for LDAP users";
    };

    extraConfig = lib.mkOption {
      type = lib.types.lines;
      default = "";
      description = "Extra configuration options for SSSD domain";
    };

    globalConfig = lib.mkOption {
      type = lib.types.lines;
      default = "";
      description = "Extra global configuration options for SSSD";
    };
  };

  config = lib.mkIf cfg.enable {
    services.sssd = {
      enable = true;
      config = ''
        [nss]
        filter_groups = root
        filter_users = root
        reconnection_retries = 3

        [sssd]
        config_file_version = 2
        reconnection_retries = 3
        domains = ${cfg.domain}
        services = nss, pam${lib.optionalString cfg.enableSSH ", ssh"}
        ${cfg.globalConfig}

        [pam]
        reconnection_retries = 3

        [domain/${cfg.domain}]
        cache_credentials = True
        id_provider = ldap
        chpass_provider = ldap
        auth_provider = ldap
        access_provider = ldap
        ldap_uri = ldaps://${cfg.server}:${toString cfg.port}

        ldap_schema = ${cfg.schema}
        ldap_search_base = ${cfg.baseDN}
        ldap_user_search_base = ${cfg.userSearch.base},${cfg.baseDN}
        ldap_group_search_base = ${
          if cfg.groupSearch.base == "" then cfg.baseDN else "${cfg.groupSearch.base},${cfg.baseDN}"
        }

        ldap_user_object_class = ${cfg.userSearch.objectClass}
        ldap_user_name = ${cfg.userSearch.nameAttribute}
        ldap_group_object_class = ${cfg.groupSearch.objectClass}
        ldap_group_name = ${cfg.groupSearch.nameAttribute}

        ${lib.optionalString cfg.accessControl.enable ''
          # Access control configuration
          ldap_access_order = filter
          ldap_access_filter = memberOf=${cfg.accessControl.allowedGroup}
        ''}

        ldap_default_bind_dn = cn=${cfg.serviceAccount.username},${cfg.userSearch.base},${cfg.baseDN}
        ldap_default_authtok_type = password
        ldap_default_authtok = file:${cfg.serviceAccount.passwordFile}

        ${cfg.extraConfig}
      '';
    };

    # Configure PAM for home directory creation
    security.pam.services = lib.mkIf cfg.createHomeDirectory {
      login.makeHomeDir = true;
      sshd.makeHomeDir = true;
      su.makeHomeDir = true;
      sudo.makeHomeDir = true;
    };

    # Disable nscd to avoid conflicts with SSSD
    services.nscd.enable = false;

    # Install useful LDAP tools
    environment.systemPackages = with pkgs; [
      openldap # provides ldapsearch, ldapwhoami, etc.
      sssd # provides sss_cache, sss_debuglevel, etc.
    ];

    # Ensure proper startup order
    systemd.services.sssd = {
      wants = [ "network-online.target" ];
      after = [ "network-online.target" ];
      wantedBy = [ "multi-user.target" ];
    };

    # Create systemd-tmpfiles rule for SSSD runtime directory
    systemd.tmpfiles.rules = [
      "d /var/lib/sss 0700 root root -"
      "d /var/lib/sss/db 0700 root root -"
      "d /var/lib/sss/deskprofile 0700 root root -"
      "d /var/lib/sss/gpo_cache 0700 root root -"
      "d /var/lib/sss/keytabs 0700 root root -"
      "d /var/lib/sss/mc 0755 root root -"
      "d /var/lib/sss/pipes 0755 root root -"
      "d /var/lib/sss/pipes/private 0700 root root -"
      "d /var/lib/sss/pubconf 0755 root root -"
      "d /var/lib/sss/secrets 0700 root root -"
    ];

    # Security warnings
    warnings =
      lib.optional (cfg.accessControl.enable && cfg.accessControl.allowedGroup == null)
        "LDAP access control is enabled but no allowed group is specified. All authenticated users will be granted access."
      ++ lib.optional (
        cfg.serviceAccount.passwordFile == null
      ) "No service account password file specified. LDAP authentication may fail.";

    # Ensure SSSD can read the password file
    systemd.services.sssd.serviceConfig = {
      SupplementaryGroups = lib.mkIf (cfg.serviceAccount.passwordFile != null) [
        (builtins.toString (builtins.dirOf cfg.serviceAccount.passwordFile))
      ];
    };
  };
}
