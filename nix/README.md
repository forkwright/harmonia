# Harmonia Nix / NixOS

Nix support for cross-compiling harmonia to aarch64-linux and deploying a
headless renderer on a Raspberry Pi with a DAC HAT.

## Build the aarch64 binary

From an x86_64-linux host with Nix installed:

```bash
nix build .#harmonia-host-aarch64
# result/bin/harmonia is an ELF aarch64-linux binary
```

Copy to the Pi:

```bash
scp result/bin/harmonia pi@raspberrypi:/usr/local/bin/harmonia
```

## Deploy via NixOS

Add the flake to your Pi's `flake.nix`:

```nix
inputs.harmonia.url = "github:forkwright/harmonia";
```

Import the modules in your Pi's `configuration.nix`:

```nix
{ inputs, ... }:
{
  imports = [
    inputs.harmonia.nixosModules.harmonia-render
    inputs.harmonia.nixosModules.harmonia-dac
  ];

  services.harmonia-render = {
    enable = true;
    name = "living-room";
    package = inputs.harmonia.packages.aarch64-linux.harmonia-host;

    # Optional: pin to a specific server instead of mDNS discovery.
    # server = "192.168.0.18:4433";

    output.device = "default";  # overridden by harmonia-dac if dac.enable = true

    dsp = {
      replayGain = "album";
      crossfeed = "none";
    };
  };

  # DAC HAT overlay (Raspberry Pi 4 only  -  see Pi 5 note below).
  services.harmonia-render.dac = {
    enable = true;
    model = "hifiberry-dacplus";  # or: hifiberry-dac2hd, iqaudio-dacplus, iqaudio-dacpro
  };
}
```

### Supported DAC models

| `dac.model` | Hardware | dt-overlay |
|---|---|---|
| `hifiberry-dacplus` | HiFiBerry DAC+ | `hifiberry-dacplus` |
| `hifiberry-dac2hd` | HiFiBerry DAC2 HD | `hifiberry-dacplusadcpro` |
| `iqaudio-dacplus` | IQaudio DAC+ | `iqaudio-dacplus` |
| `iqaudio-dacpro` | IQaudio DAC Pro | `iqaudio-dacplus,auto` |

The `harmonia-dac` module:
- Enables the device tree overlay via `hardware.raspberry-pi."4".dt-overlays`.
- Disables onboard BCM audio (`hardware.raspberry-pi."4".audio.enable = false`).
- Writes `/etc/asound.conf` to set the DAC as the ALSA default card.
- Sets `services.harmonia-render.output.device` to the DAC's ALSA card (overridable).

### Pi 5 note

Pi 5 uses a different firmware config format (`config.txt` sections differ).
The `hardware.raspberry-pi."4"` module does not apply to Pi 5.
For Pi 5, set overlays manually via `boot.kernelParams` or the Pi 5-specific
NixOS module when it stabilises in nixpkgs. The `harmonia-render` service
module works on Pi 5 without change; only `harmonia-dac` needs adjustment.

## Pairing the renderer

1. Start harmonia server on the host machine.
2. Start the renderer on the Pi (`systemctl start harmonia-render`).
3. mDNS discovery finds the server automatically on the same LAN.
4. On first run, the renderer stores the server's TLS fingerprint in `certDir`
   (TOFU  -  Trust On First Use). Subsequent reconnects verify the fingerprint.
5. If the server is on a different subnet, set `services.harmonia-render.server`.

## Troubleshooting

**ALSA device not found**

```bash
aplay -l           # list ALSA cards
aplay -D hw:0 /dev/zero   # test playback on card 0
```

Check `/etc/asound.conf` matches the detected card name. The card name shown
by `aplay -l` (e.g. `sndrpihifiberry`) must match what the module writes.

**DAC not detected**

```bash
dtoverlay -l       # list active overlays
vcgencmd get_config int | grep audio  # check onboard audio state
dmesg | grep hifiberry  # or iqaudio
```

Ensure only one audio overlay is active. Remove conflicting overlays.

**Renderer not connecting**

```bash
journalctl -u harmonia-render -f     # live logs
systemctl status harmonia-render     # unit status + last error

# Verify mDNS is working:
avahi-browse _harmonia._tcp --terminate

# If mDNS fails, pin the server address:
# services.harmonia-render.server = "192.168.x.x:4433";
```

**Watchdog killing the service**

The service is `Type=notify`. It sends `READY=1` after initialization.
If startup takes longer than `WatchdogSec=30` (e.g. slow mDNS or network),
increase `systemd.services.harmonia-render.serviceConfig.WatchdogSec`.
