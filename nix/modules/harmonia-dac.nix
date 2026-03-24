{ config, lib, ... }:

# DAC HAT device tree overlay configuration for Raspberry Pi.
# Extends services.harmonia-render with a dac sub-option that selects
# the correct dt-overlay and sets the ALSA default card.

let
  cfg = config.services.harmonia-render;
  dacCfg = cfg.dac;

  # Map model names to device tree overlay identifiers.
  # Sources: HiFiBerry docs, IQaudio docs, Raspberry Pi overlay index.
  overlayMap = {
    # HiFiBerry DAC+ (standard, non-HD)
    hifiberry-dacplus    = { overlay = "hifiberry-dacplus";        alsaCard = "sndrpihifiberry"; };
    # HiFiBerry DAC2 HD uses the "adcpro" overlay (same silicon, extra ADC channels)
    hifiberry-dac2hd     = { overlay = "hifiberry-dacplusadcpro";  alsaCard = "sndrpihifiberry"; };
    # IQaudio DAC+ (standard)
    iqaudio-dacplus      = { overlay = "iqaudio-dacplus";          alsaCard = "IQaudIODAC"; };
    # IQaudio DAC Pro uses the same overlay with auto-detection flag
    iqaudio-dacpro       = { overlay = "iqaudio-dacplus,auto";     alsaCard = "IQaudIODAC"; };
  };

  selected = overlayMap.${dacCfg.model};

in {
  options.services.harmonia-render.dac = {
    enable = lib.mkEnableOption "DAC HAT device tree overlay";

    model = lib.mkOption {
      type = lib.types.enum (builtins.attrNames overlayMap);
      description = ''
        DAC HAT model. Selects the correct device tree overlay and ALSA card name.

        Supported models:
          hifiberry-dacplus   — HiFiBerry DAC+
          hifiberry-dac2hd    — HiFiBerry DAC2 HD
          iqaudio-dacplus     — IQaudio DAC+
          iqaudio-dacpro      — IQaudio DAC Pro
      '';
      example = "hifiberry-dacplus";
    };
  };

  config = lib.mkIf (cfg.enable && dacCfg.enable) {
    # Enable the DAC overlay and disable onboard audio on Pi 4.
    # Pi 5 uses a compatible mechanism — see nix/README.md for Pi 5 notes.
    hardware.raspberry-pi."4" = {
      dt-overlays.${selected.overlay}.enable = true;

      # WHY: Onboard BCM audio and the DAC HAT share the I2S bus;
      # both cannot run simultaneously.
      audio.enable = false;
    };

    # Set the DAC as the ALSA default card system-wide.
    environment.etc."asound.conf".text = ''
      pcm.!default {
        type hw
        card ${selected.alsaCard}
      }
      ctl.!default {
        type hw
        card ${selected.alsaCard}
      }
    '';

    # Propagate the DAC ALSA device into the renderer output config when
    # the user has not explicitly set a device.
    services.harmonia-render.output.device = lib.mkDefault "hw:${selected.alsaCard}";
  };
}
