# Spec 02: Desktop App (Tauri)

**Status:** Active
**Priority:** High
**Issues:** #43, #44, #45, #46, #80

## Goal

Ship a Linux desktop app using Tauri. The web frontend already exists — Tauri wraps it with native audio output (PipeWire/ALSA for bit-perfect), media key integration (MPRIS D-Bus), and system tray. This is the path to bit-perfect playback on desktop, which browsers can't deliver.

## Phases

### Phase 1: Tauri shell
- [ ] Tauri 2 build with existing React frontend
- [ ] Dev workflow (hot reload, mock API)
- [ ] Window management, system tray

### Phase 2: Native audio
- [ ] PipeWire audio output (Rust, via akroasis-core FFI)
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

- Tauri 2 stable (available)
- akroasis-core FFI bindings (JNI exists, FFI layer started in shared/)

## Notes

- Tauri was already chosen over Qt6/GTK4 (#43). React + Rust backend is the stack.
- Sony Walkman runs Android, so desktop is secondary platform but important for the audiophile positioning.
- Multi-zone playback (#80) deferred — requires significant Mouseion backend work.
