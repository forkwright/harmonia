# Spec 02: Desktop App (Tauri)

**Status:** Active
**Priority:** High
**Issues:** #44, #45, #46, #80

## Goal

Ship a Linux desktop app using Tauri. The web frontend already exists — Tauri wraps it with native audio output (PipeWire/ALSA for bit-perfect), media key integration (MPRIS D-Bus), and system tray. This is the path to bit-perfect playback on desktop, which browsers can't deliver.

## Phases

### Phase 1: Tauri shell (DONE)
- [x] Tauri 2 build with existing React frontend — PR #149
- [x] Window management, system tray, close-to-tray — PR #149
- [x] Platform detection (`isTauri()`) — PR #149
- [x] CSP for remote Mouseion connections — PR #149
- [x] GUI framework chosen: Tauri 2.3 (#43 closed)

### Phase 2: Native audio
- [ ] PipeWire audio output (Rust, via akroasis-core FFI) (#44)
- [ ] ALSA fallback for systems without PipeWire
- [ ] Bit-perfect mode (exclusive device access)
- [ ] ReplayGain processing via akroasis-core

### Phase 3: Desktop integration
- [ ] MPRIS D-Bus for media keys (#45)
- [ ] Desktop notifications
- [ ] File associations (open .flac, .m4b, .epub)

### Phase 4: Packaging & distribution
- [ ] AppImage (primary)
- [ ] Flatpak
- [ ] .deb
- [ ] AUR package (#46)

## Dependencies

- akroasis-core FFI bindings (JNI exists, FFI layer started in shared/)

## Notes

- Tauri shell shipped in PR #149 with system tray and close-to-tray.
- akroasis-core dependency commented out in Tauri Cargo.toml — uncomment when FFI is ready.
- Multi-zone playback (#80) deferred — requires Mouseion real-time sync protocol.
