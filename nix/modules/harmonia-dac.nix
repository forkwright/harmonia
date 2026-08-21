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
    hifiberry-dacplus    = { overlayFile = "hifiberry-dacplus";       alsaCard = "sndrpihifiberry"; };
    # HiFiBerry DAC2 HD uses the "adcpro" overlay (same silicon, extra ADC channels)
    hifiberry-dac2hd     = { overlayFile = "hifiberry-dacplusadcpro"; alsaCard = "sndrpihifiberry"; };
    # IQaudio DAC+ (standard)
    iqaudio-dacplus      = { overlayFile = "iqaudio-dacplus";         alsaCard = "IQaudIODAC"; };
    # IQaudio DAC Pro — same base overlay. Upstream recommends the ",auto"
    # parameter for Pro auto-detection; hardware.deviceTree.overlays cannot
    # express overlay parameters, so the Pro is driven by the base overlay
    # and detection should be verified on the hardware.
    iqaudio-dacpro       = { overlayFile = "iqaudio-dacplus";         alsaCard = "IQaudIODAC"; };
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
    # Current-nixpkgs Pi device-tree configuration: the
    # hardware.raspberry-pi."4" namespace was removed upstream, so the DAC
    # overlay is applied through the generic hardware.deviceTree mechanism
    # (the dtbo ships in the rpi kernel's dtbs/overlays directory).
    hardware.deviceTree = {
      enable = true;
      overlays = [{
        name = selected.overlayFile;
        dtboFile = "${config.boot.kernelPackages.kernel}/dtbs/overlays/${selected.overlayFile}.dtbo";
      }];
    };

    # WHY: onboard BCM audio and the DAC HAT share the I2S bus and cannot
    # run simultaneously; the removed hardware.raspberry-pi audio switch
    # did this by withholding the overlay, and the current-API equivalent
    # is to keep the onboard audio driver from loading.
    boot.blacklistedKernelModules = [ "snd_bcm2835" ];

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
