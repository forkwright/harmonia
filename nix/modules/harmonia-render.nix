{ config, lib, pkgs, ... }:

let
  cfg = config.services.harmonia-render;
  settingsFormat = pkgs.formats.toml { };

  # Merge module options into a RendererConfig-compatible TOML structure.
  rendererSettings = {
    output = {
      device = cfg.output.device;
    } // lib.optionalAttrs (cfg.output.sampleRate != null) {
      sample_rate = cfg.output.sampleRate;
    };

    dsp = {
      volume.level_db = 20.0 * (builtins.log cfg.dsp.volume / builtins.log 10.0);
      replaygain = {
        enabled = cfg.dsp.replayGain != "off";
        mode = if cfg.dsp.replayGain == "album" then "album" else "track";
      };
      crossfeed = {
        enabled = cfg.dsp.crossfeed != "none";
        strength = if cfg.dsp.crossfeed == "relaxed" then 0.2
                   else if cfg.dsp.crossfeed == "natural" then 0.3
                   else 0.0;
      };
      eq = {
        enabled = cfg.dsp.eq != [];
        bands = map (b: { frequency = b.freq; gain_db = b.gain; q = b.q; }) cfg.dsp.eq;
      };
    };
  };

  generatedConfig = settingsFormat.generate "harmonia-render.toml" rendererSettings;

  activeConfig = if cfg.configFile != null then cfg.configFile else generatedConfig;

  renderArgs = lib.escapeShellArgs (
    [ "--name" cfg.name "--config" (toString activeConfig) "--cert-dir" cfg.certDir ]
    ++ lib.optionals (cfg.server != null) [ "--server" cfg.server ]
  );
in {
  options.services.harmonia-render = {
    enable = lib.mkEnableOption "Harmonia renderer (headless audio endpoint for a DAC HAT)";

    package = lib.mkOption {
      type = lib.types.package;
      description = "Harmonia package to use.";
    };

    name = lib.mkOption {
      type = lib.types.str;
      description = "Renderer display name shown in the server UI.";
      example = "living-room";
    };

    server = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Server address (host:port). Omit to use mDNS discovery.";
      example = "192.168.0.18:4433";
    };

    certDir = lib.mkOption {
      type = lib.types.path;
      default = "/var/lib/harmonia-render/certs";
      description = "Directory for TLS certificates and pairing credentials.";
    };

    configFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = "Path to a renderer TOML config. When set, overrides generated config.";
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "harmonia";
      description = "User to run the renderer as.";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "harmonia";
      description = "Group to run the renderer as.";
    };

    output = {
      device = lib.mkOption {
        type = lib.types.str;
        default = "default";
        description = "ALSA device name (e.g. 'default', 'hw:0', 'hw:Headphones').";
      };

      sampleRate = lib.mkOption {
        type = lib.types.nullOr lib.types.int;
        default = null;
        description = "Sample rate override in Hz. Null lets the driver negotiate.";
        example = 48000;
      };
    };

    dsp = {
      volume = lib.mkOption {
        type = lib.types.float;
        default = 1.0;
        description = "Linear volume multiplier (1.0 = 0 dBFS, 0.5 = -6 dBFS).";
      };

      replayGain = lib.mkOption {
        type = lib.types.enum [ "off" "track" "album" ];
        default = "album";
        description = "ReplayGain mode applied to the output stream.";
      };

      crossfeed = lib.mkOption {
        type = lib.types.enum [ "none" "natural" "relaxed" ];
        default = "none";
        description = "Headphone crossfeed preset.";
      };

      eq = lib.mkOption {
        type = lib.types.listOf (lib.types.submodule {
          options = {
            freq = lib.mkOption { type = lib.types.float; description = "Center frequency in Hz."; };
            gain = lib.mkOption { type = lib.types.float; description = "Gain in dBFS (negative = cut)."; };
            q    = lib.mkOption { type = lib.types.float; default = 1.414; description = "Q factor (bandwidth)."; };
          };
        });
        default = [];
        description = "Parametric EQ bands applied to the output.";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.group;
      home = "/var/lib/harmonia-render";
    };
    users.groups.${cfg.group} = { };

    systemd.services.harmonia-render = {
      description = "Harmonia Renderer";
      after = [ "network-online.target" "sound.target" ];
      wants = [ "network-online.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "notify";
        User = cfg.user;
        Group = cfg.group;
        ExecStart = "${cfg.package}/bin/harmonia ${renderArgs}";
        Restart = "always";
        RestartSec = 5;
        WatchdogSec = 30;

        StateDirectory = "harmonia-render";
        StateDirectoryMode = "0750";
        RuntimeDirectory = "harmonia-render";

        # Hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = false;  # WHY: audio devices must remain accessible
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictNamespaces = true;
        RestrictRealtime = false; # WHY: ALSA and real-time audio scheduling require this
        RestrictSUIDSGID = true;
        LockPersonality = true;
        SystemCallFilter = [ "@system-service" "~@privileged" ];
        SystemCallArchitectures = "native";

        ReadWritePaths = [ cfg.certDir ];
        SupplementaryGroups = [ "audio" ];
      };
    };

    # Ensure cert directory exists with correct ownership.
    systemd.tmpfiles.rules = [
      "d '${cfg.certDir}' 0750 ${cfg.user} ${cfg.group} - -"
    ];
  };
}
