# Manual Testing Checklist

Comprehensive manual testing for Akroasis Android app covering Phase 1, 3, 6, 7 features.

## Phase 0: Foundation

### Audio Pipeline
- [ ] **Bit-perfect playback** - Verify 16/24/32-bit audio plays without degradation
- [ ] **Sample rate handling** - Test 44.1kHz, 48kHz, 96kHz, 192kHz files
- [ ] **Format support** - FLAC, ALAC, WAV, MP3, AAC, Opus playback
- [ ] **USB DAC detection** - Connect/disconnect USB DAC, verify audio routing
- [ ] **Audio focus** - Test interruptions (calls, notifications, other apps)

## Phase 1: Playback Excellence

### Signal Path Visualization
- [ ] **Pipeline display** - View active DSP components in NowPlayingScreen
- [ ] **Real-time updates** - Enable/disable effects, verify UI updates
- [ ] **Sample rate display** - Verify correct sample rate shown
- [ ] **Bit depth display** - Verify correct bit depth shown

### Gapless Playback
- [ ] **Gapless albums** - Play album, verify <50ms gaps between tracks
- [ ] **Gap verification UI** - Access gapless settings, view measurements
- [ ] **Gap history** - Verify gap measurements are tracked
- [ ] **Clear measurements** - Test clearing gap history
- [ ] **Rapid skip** - Skip through tracks quickly, verify no crashes

### Playback Speed Memory
- [ ] **Track-specific speed** - Set speed for track, verify persists on replay
- [ ] **Album-level speed** - Set speed for album, verify all tracks use it
- [ ] **Priority fallback** - Track speed > Album speed > Default (1.0x)
- [ ] **Audiobook speed** - Set different speed for audiobook content
- [ ] **Speed persistence** - Close/reopen app, verify speeds preserved

### Queue Management
- [ ] **Add to queue** - Add tracks, verify order
- [ ] **Remove from queue** - Swipe-to-remove, verify works
- [ ] **Drag to reorder** - Long-press and drag, verify reordering
- [ ] **Clear queue** - Clear all tracks
- [ ] **Queue history (undo)** - Add track, undo, verify removed
- [ ] **Queue history (redo)** - Undo then redo, verify restored
- [ ] **History limit** - Make 60+ changes, verify undo limit at 50
- [ ] **Shuffle** - Enable shuffle, verify random playback
- [ ] **Repeat modes** - Test OFF/ONE/ALL repeat modes

### Queue Export
- [ ] **M3U export** - Export queue to M3U, verify file structure
- [ ] **M3U8 export** - Export to M3U8, verify extended metadata
- [ ] **PLS export** - Export to PLS, verify format
- [ ] **Empty queue export** - Export empty queue, verify no crash
- [ ] **Large queue export** - Export 100+ tracks, verify performance
- [ ] **UTF-8 support** - Export tracks with non-ASCII characters

## Phase 3: DSP Engine

### Parametric EQ
- [ ] **Enable/disable EQ** - Toggle EQ, verify audio change
- [ ] **5-band adjustment** - Adjust each band, verify effect
- [ ] **Frequency display** - Verify correct frequencies shown
- [ ] **Gain range** - Test -12dB to +12dB range
- [ ] **Preset loading** - Load each preset, verify settings applied

### AutoEQ Profiles
- [ ] **HD 600 profile** - Load, verify EQ curve applied
- [ ] **HD 650 profile** - Load, verify EQ curve applied
- [ ] **DT 770 Pro profile** - Load, verify EQ curve applied
- [ ] **ATH-M50x profile** - Load, verify EQ curve applied
- [ ] **Profile search** - Search for "Sennheiser", verify results
- [ ] **Profile switching** - Switch between profiles, verify curves change

### Crossfeed
- [ ] **Enable/disable crossfeed** - Toggle, verify stereo width change
- [ ] **Low strength (15%)** - Test subtle crossfeed
- [ ] **Medium strength (30%)** - Test moderate crossfeed
- [ ] **High strength (50%)** - Test strong crossfeed
- [ ] **Mono audio bypass** - Play mono file, verify no processing

### Headroom Management
- [ ] **Enable/disable headroom** - Toggle, verify volume change
- [ ] **-3dB headroom** - Set moderate, verify attenuation
- [ ] **-6dB headroom** - Set safe, verify attenuation
- [ ] **-12dB headroom** - Set maximum, verify significant reduction
- [ ] **0dB headroom** - Set none, verify unity gain
- [ ] **Clipping detection** - Play loud track, verify indicator
- [ ] **Peak level display** - Verify real-time peak monitoring
- [ ] **Reset indicator** - Clear clipping indicator, verify cleared
- [ ] **Recommended headroom** - Enable EQ+crossfeed, verify suggestion

### DSP Combinations
- [ ] **EQ + Crossfeed** - Enable both, verify no artifacts
- [ ] **EQ + Headroom** - Boost EQ, verify headroom prevents clipping
- [ ] **Crossfeed + Headroom** - Enable both, verify smooth operation
- [ ] **All DSP effects** - Enable EQ + Crossfeed + Headroom, verify quality
- [ ] **Effect order** - Verify DSP chain: Crossfeed → Headroom → EQ

## Phase 6: Mobile Optimization

### Media Session
- [ ] **Notification controls** - Play/pause/skip from notification
- [ ] **Lock screen controls** - Control playback from lock screen
- [ ] **Now playing info** - Verify track title/artist/album shown
- [ ] **Artwork display** - Verify album art shown in notification
- [ ] **Media button** - Test hardware play/pause button

### Playback Notification
- [ ] **Notification persistence** - Verify notification stays during playback
- [ ] **Notification actions** - Test all action buttons
- [ ] **Auto-dismiss** - Stop playback, verify notification removed
- [ ] **Tap to open** - Tap notification, verify app opens

### State Persistence
- [ ] **Resume on restart** - Close app mid-track, reopen, verify position restored
- [ ] **Queue restoration** - Close app with queue, reopen, verify queue intact
- [ ] **Playback speed restore** - Close app, reopen, verify speed preserved
- [ ] **Repeat/shuffle restore** - Close app, reopen, verify modes preserved
- [ ] **Position accuracy** - Resume, verify within 1 second of stop point

### Network Monitoring
- [ ] **WiFi detection** - Connect to WiFi, verify detected
- [ ] **Cellular detection** - Switch to cellular, verify detected
- [ ] **Network change** - Switch networks, verify transition smooth
- [ ] **Offline mode** - Disable network, verify local playback continues
- [ ] **Metered warning** - On cellular, verify quality adjustment prompt

### Battery Optimization
- [ ] **Low battery detection** - Drain to 20%, verify warning
- [ ] **Quality reduction** - On low battery, verify DSP effects disable option
- [ ] **Battery impact estimate** - View settings, verify hours remaining shown
- [ ] **Charging detection** - Plug in charger, verify full quality restored

## Phase 7: Discovery & Scrobbling

### Last.fm Integration
- [ ] **Authentication** - Log in to Last.fm, verify success
- [ ] **Now playing update** - Start track, verify Last.fm shows "now playing"
- [ ] **Scrobble submission** - Play >50% of track, verify scrobbled
- [ ] **Scrobble timestamp** - Verify scrobble time matches playback start
- [ ] **Speed adjustment** - Play at 1.5x, verify timestamp accounts for speed
- [ ] **Connection status** - Disconnect from Last.fm, verify indicator
- [ ] **Error handling** - Scrobble with no internet, verify retry

### ListenBrainz Integration
- [ ] **Token authentication** - Add ListenBrainz token, verify auth
- [ ] **Listen submission** - Play track, verify listen recorded
- [ ] **Concurrent scrobbling** - Enable both services, verify both receive
- [ ] **ListenBrainz status** - View connection status
- [ ] **Token validation** - Enter invalid token, verify error shown

### Scrobble Settings
- [ ] **Enable/disable services** - Toggle each service independently
- [ ] **Scrobble threshold** - Verify 50% or 240s rule
- [ ] **Manual scrobble** - Force scrobble, verify submission
- [ ] **Scrobble history** - View recent scrobbles
- [ ] **Clear cache** - Clear scrobble cache, verify emptied

## Cross-Feature Integration

### Complete Listening Session
- [ ] **Start to finish** - Play album with all DSP effects, scrobbling, and queue
- [ ] **Mid-session changes** - Change EQ mid-track, verify smooth transition
- [ ] **Speed during scrobble** - Change speed, verify scrobble timestamp correct
- [ ] **Queue export during playback** - Export queue while playing, verify no interruption

### Error Recovery
- [ ] **Force close** - Force close app, reopen, verify graceful recovery
- [ ] **Network loss** - Lose connection mid-scrobble, verify retry
- [ ] **Low memory** - Play with low available memory, verify no crash
- [ ] **Corrupted cache** - Clear app data, verify rebuild

### Performance
- [ ] **Large library** - Test with 10,000+ tracks
- [ ] **Long queue** - Add 500+ tracks to queue, verify performance
- [ ] **Rapid interactions** - Quickly skip/adjust settings, verify responsiveness
- [ ] **Background playback** - Play for 30+ minutes in background
- [ ] **Battery drain** - Monitor battery usage over 1 hour

## UI/UX

### Accessibility
- [ ] **Large text** - Enable system large text, verify readability
- [ ] **Dark mode** - Switch to dark mode, verify contrast
- [ ] **Touch targets** - Verify all buttons easily tappable (44dp min)
- [ ] **Screen reader** - Test with TalkBack, verify labels

### Error Messages
- [ ] **File not found** - Delete track file, verify clear error
- [ ] **Network timeout** - Simulate timeout, verify error message
- [ ] **Invalid format** - Add corrupt file, verify error shown
- [ ] **Permission denied** - Revoke storage permission, verify prompt

### Edge Cases
- [ ] **0 byte file** - Add empty file, verify handled gracefully
- [ ] **Extremely long track** - Play 10+ hour file, verify UI handles
- [ ] **Special characters** - Track name with emoji/symbols, verify display
- [ ] **No metadata** - Play file with no tags, verify defaults shown

---

## Test Environment

**Device:** _________________________
**Android Version:** _________________
**App Version:** ____________________
**Date:** ___________________________
**Tester:** __________________________

## Notes

Use this section to document bugs, observations, or unexpected behavior:

---

## Sign-off

- [ ] All critical features tested and working
- [ ] No P0/P1 bugs found
- [ ] Performance acceptable on test device
- [ ] Ready for release

**Tester Signature:** _______________________
**Date:** __________________________________
