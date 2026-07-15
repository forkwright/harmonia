{ config, lib, pkgs, ... }:

let
  cfg = config.services.harmonia;
  desktopCfg = config.programs.harmonia-desktop;
  settingsFormat = pkgs.formats.toml { };
  configFile = settingsFormat.generate "harmonia.toml" cfg.settings;

  # WHY: these mirror crates/horismos/src/subsystems.rs ErgasiaConfig::download_dir
  # and KomideConfig::podcast_dir compiled-in defaults. The Nix module has no
  # way to introspect the Rust binary's Default impl at eval time, so these
  # two literals must be kept in lockstep by hand if either Rust default
  # changes. Both directories are hard boot requirements: archon's
  # validate_download_dir (crates/archon/src/serve.rs) rejects startup if
  # ergasia.download_dir does not exist or is not writable, and ProtectSystem
  # = strict below means neither directory is reachable unless it is also
  # named in ReadWritePaths.
  downloadDir = cfg.settings.ergasia.download_dir or "/data/downloads";
  podcastDir = cfg.settings.komide.podcast_dir or "/data/podcasts";
in {
  options.programs.harmonia-desktop = {
    enable = lib.mkEnableOption "Harmonia desktop application";

    package = lib.mkOption {
      type = lib.types.package;
      description = "Harmonia desktop package to use.";
    };

    installMimeTypes = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Register Harmonia as a handler for common audio MIME types.";
    };
  };

  options.services.harmonia = {
    enable = lib.mkEnableOption "Harmonia media server";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.harmonia;
      description = "Harmonia package to use.";
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "harmonia";
      description = "User to run harmonia as.";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "harmonia";
      description = "Group to run harmonia as.";
    };

    dataDir = lib.mkOption {
      type = lib.types.path;
      default = "/var/lib/harmonia";
      description = "State directory (database, cache).";
    };

    settings = lib.mkOption {
      type = settingsFormat.type;
      default = { };
      description = "Configuration written to harmonia.toml.";
      example = lib.literalExpression ''
        {
          paroche.port = 8096;
          # horismos::LibraryConfig (crates/horismos/src/subsystems.rs) has no
          # per-field defaults — every field below is required or config
          # parsing fails at startup.
          taxis.libraries.music = {
            path = "/media/music";
            media_type = "music";
            watcher_mode = "auto";
            poll_interval_seconds = 300;
            scan_interval_hours = 24;
          };
        }
      '';
    };

    secretsFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "Path to secrets file (JWT secret, API keys). Integrate with agenix or sops-nix.";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open firewall port for Harmonia.";
    };
  };

  config = lib.mkIf cfg.enable {
    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.group;
      home = cfg.dataDir;
    };
    users.groups.${cfg.group} = { };

    # WHY: neither directory is created automatically the way `StateDirectory`
    # creates dataDir — without this, a fresh instance fails
    # validate_download_dir at first boot (directory does not exist).
    # Parent directories are created implicitly by systemd-tmpfiles.
    systemd.tmpfiles.rules = [
      "d ${downloadDir} 0750 ${cfg.user} ${cfg.group} - -"
      "d ${podcastDir} 0750 ${cfg.user} ${cfg.group} - -"
    ];

    systemd.services.harmonia = {
      description = "Harmonia Media Server";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        User = cfg.user;
        Group = cfg.group;
        ExecStart = "${cfg.package}/bin/harmonia serve --config ${configFile}";
        Restart = "on-failure";
        RestartSec = 5;
        StateDirectory = "harmonia";
        StateDirectoryMode = "0750";

        # Systemd hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        MemoryDenyWriteExecute = true;
        LockPersonality = true;
        SystemCallFilter = [ "@system-service" "~@privileged" ];
        SystemCallArchitectures = "native";

        ReadWritePaths = [
          cfg.dataDir
          downloadDir
          podcastDir
        ] ++ (lib.mapAttrsToList (_: lib.getAttr "path") (cfg.settings.taxis.libraries or { }));

        LoadCredential = lib.mkIf (cfg.secretsFile != null) [
          "secrets.toml:${cfg.secretsFile}"
        ];
      };

      environment = {
        HARMONIA__DATABASE__DB_PATH = "${cfg.dataDir}/harmonia.db";
      } // lib.optionalAttrs (cfg.secretsFile != null) {
        # WHY: horismos::secrets::secrets_path (crates/horismos/src/secrets.rs)
        # honors this env var as an override for the config-sibling default —
        # required because ${configFile} lives in the read-only Nix store,
        # which has no writable sibling secrets.toml.
        HARMONIA_SECRETS_PATH = "%d/secrets.toml";
      };
    };

    networking.firewall.allowedTCPPorts =
      lib.mkIf cfg.openFirewall [ (cfg.settings.paroche.port or 8096) ];
  } // lib.mkIf desktopCfg.enable {
    # Desktop application launcher entry.
    xdg.desktopEntries.harmonia = {
      name = "Harmonia";
      genericName = "Music Player";
      comment = "Self-hosted music, podcasts, and audiobooks";
      exec = "${desktopCfg.package}/bin/harmonia %U";
      icon = "harmonia";
      categories = [ "Audio" "Music" "Player" "AudioVideo" ];
      startupNotify = true;
      mimeType = lib.optionals desktopCfg.installMimeTypes [
        "audio/flac"
        "audio/mpeg"
        "audio/mp4"
        "audio/ogg"
        "audio/opus"
        "audio/wav"
        "audio/aac"
        "x-scheme-handler/harmonia"
      ];
    };

    # D-Bus service file — allows the desktop environment to activate
    # Harmonia for MPRIS without it already running.
    services.dbus.packages = [ desktopCfg.package ];

    # MIME type associations (xdg-mime).
    xdg.mime.defaultApplications = lib.mkIf desktopCfg.installMimeTypes {
      "audio/flac" = "harmonia.desktop";
      "audio/mpeg" = "harmonia.desktop";
      "audio/mp4" = "harmonia.desktop";
      "audio/ogg" = "harmonia.desktop";
      "audio/opus" = "harmonia.desktop";
      "audio/wav" = "harmonia.desktop";
      "audio/aac" = "harmonia.desktop";
    };
  };
}
