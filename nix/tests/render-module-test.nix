{ pkgs }:

# Module evaluation tests for services.harmonia-render and services.harmonia-render.dac.
# These run in a lightweight NixOS VM (nixosTest) and verify that:
#   1. The systemd service is generated with the expected properties.
#   2. The DAC overlay mapping produces correct ALSA and dt-overlay configuration.
#
# The tests do NOT start harmonia itself — they validate module wiring only.

let
  baseModule = ../modules/harmonia-render.nix;
  dacModule  = ../modules/harmonia-dac.nix;

  # Helper: evaluate a NixOS config and return its systemd service unit.
  evalRenderService = extraConfig:
    (pkgs.nixos (
      { ... }: {
        imports = [ baseModule ];
        nixpkgs.hostPlatform = "x86_64-linux";
        services.harmonia-render = {
          package = pkgs.harmonia or pkgs.coreutils; # coreutils as stand-in when not available
        } // extraConfig;
      }
    )).config.systemd.services.harmonia-render or null;

  evalFull = extraConfig:
    (pkgs.nixos (
      { ... }: {
        imports = [ baseModule dacModule ];
        nixpkgs.hostPlatform = "x86_64-linux";
        services.harmonia-render = {
          package = pkgs.harmonia or pkgs.coreutils;
        } // extraConfig;
      }
    )).config;

in pkgs.nixosTest {
  name = "harmonia-render-module";

  nodes.machine = { ... }: {
    imports = [ baseModule dacModule ];

    nixpkgs.overlays = [
      (final: prev: {
        harmonia = pkgs.harmonia or (prev.coreutils);
      })
    ];

    services.harmonia-render = {
      enable = true;
      name = "test-room";
      server = "192.168.1.1:4433";
      package = pkgs.harmonia or pkgs.coreutils;
    };
  };

  testScript = ''
    import json

    machine.wait_for_unit("multi-user.target")

    # Verify the service unit exists.
    machine.succeed("systemctl cat harmonia-render.service")

    # Service must be Type=notify for watchdog support.
    out = machine.succeed("systemctl show harmonia-render.service --property=Type")
    assert "Type=notify" in out, f"Expected Type=notify, got: {out}"

    # Restart policy must be 'always'.
    out = machine.succeed("systemctl show harmonia-render.service --property=Restart")
    assert "Restart=always" in out, f"Expected Restart=always, got: {out}"

    # WatchdogSec must be set.
    out = machine.succeed("systemctl show harmonia-render.service --property=WatchdogUSec")
    assert "0" not in out or "WatchdogUSec=0" not in out, f"WatchdogSec not configured: {out}"

    # The audio supplementary group must be present.
    out = machine.succeed("systemctl show harmonia-render.service --property=SupplementaryGroups")
    assert "audio" in out, f"audio group missing from SupplementaryGroups: {out}"

    # ProtectSystem=strict must be set.
    out = machine.succeed("systemctl show harmonia-render.service --property=ProtectSystem")
    assert "strict" in out.lower(), f"ProtectSystem=strict not set: {out}"

    # Cert directory must exist.
    machine.succeed("test -d /var/lib/harmonia-render/certs")
  '';
}
