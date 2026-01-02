# Phase 1 QA Test Plan

## Features to Test

### 1. Variable Speed Playback (0.5x-3.0x)
- [ ] Preset buttons work (0.5x, 0.75x, 1.0x, 1.25x, 1.5x, 2.0x, 2.5x, 3.0x)
- [ ] Slider works smoothly
- [ ] Speed changes applied immediately
- [ ] Speed persists across app restarts
- [ ] Display shows correct speed (%.2f format)
- [ ] Signal path shows speed when != 1.0x

### 2. Sleep Timer
- [ ] Duration presets work (15/30/45/60 min)
- [ ] End-of-track mode works correctly
- [ ] Timer counts down properly
- [ ] Fade out works (3 seconds)
- [ ] Playback stops after fade
- [ ] Cancel timer works
- [ ] Timer display updates every second
- [ ] Timer state survives screen rotation

### 3. Queue Management
- [ ] Queue screen opens from Now Playing
- [ ] Swipe-to-remove works (right-to-left)
- [ ] Current track highlighted correctly
- [ ] Tap track to play works
- [ ] Queue updates when tracks added/removed
- [ ] Empty queue shows placeholder
- [ ] Queue count displayed correctly

### 4. Signal Path Visualization
- [ ] Shows/hides with toggle button
- [ ] Source format displayed
- [ ] Decode format shown (kHz/bit)
- [ ] Process stage shows active effects
- [ ] Output shows USB DAC when connected
- [ ] Bit-perfect indicator correct
- [ ] Gapless status displayed
- [ ] Channel count shown
- [ ] Updates real-time when settings change

### 5. Gapless Playback Verification
- [ ] Gapless status shows in signal path
- [ ] Status updates when toggled in settings
- [ ] Color coding correct (primary when enabled)

### 6. Battery-Aware Playback
- [ ] Battery level monitored (check every 30s)
- [ ] Low battery warning appears at 20%
- [ ] EQ auto-disabled when low battery
- [ ] EQ re-enabled when battery recovers
- [ ] No effect while charging
- [ ] Battery impact estimate displayed
- [ ] Estimate updates with setting changes

### 7. A/B Testing Mode
- [ ] A/B card appears when in mode
- [ ] Version indicator shows A or B
- [ ] Switch button works
- [ ] Position preserved when switching
- [ ] Exit button works
- [ ] Can't switch when not in A/B mode
- [ ] State cleared on exit

## Integration Tests

### Feature Interactions
- [ ] Signal path updates when battery disables EQ
- [ ] Battery estimate changes with EQ/speed/DAC
- [ ] Sleep timer works with A/B mode
- [ ] Queue works with A/B mode
- [ ] Speed changes reflected in signal path
- [ ] All features work while charging
- [ ] Multiple features active simultaneously

### State Management
- [ ] Settings persist across app restarts
- [ ] PlaybackState updates correctly
- [ ] StateFlows emit properly
- [ ] No memory leaks with coroutines
- [ ] ViewModel cleanup on destroy

### UI/UX
- [ ] No UI jank or stuttering
- [ ] Buttons responsive
- [ ] Indicators update immediately
- [ ] Error states handled gracefully
- [ ] Loading states shown when needed
- [ ] Dark mode support
- [ ] Landscape orientation support

## Edge Cases

### Variable Speed
- [ ] Minimum speed (0.5x) works
- [ ] Maximum speed (3.0x) works
- [ ] Speed change during playback
- [ ] Speed with very long tracks
- [ ] Speed with very short tracks

### Sleep Timer
- [ ] Timer with < 1 minute remaining
- [ ] Multiple timer starts/cancels
- [ ] Timer during seek operations
- [ ] End-of-track with 1 second left
- [ ] Timer with repeat mode enabled

### Queue Management
- [ ] Remove last track in queue
- [ ] Remove currently playing track
- [ ] Swipe gesture conflicts
- [ ] Very long queue (100+ tracks)
- [ ] Queue with duplicate tracks

### Battery-Aware
- [ ] Battery exactly at threshold (20%)
- [ ] Rapid battery changes
- [ ] Charging state changes
- [ ] EQ already disabled when battery low
- [ ] Multiple effects (EQ + speed + DAC)

### A/B Testing
- [ ] Same track as both A and B
- [ ] Very different track lengths
- [ ] Switch during loading
- [ ] Switch at track boundaries
- [ ] Exit during playback

## Code Quality Checks

### Null Safety
- [ ] No !! operators on network/external data
- [ ] Safe calls (?.) used appropriately
- [ ] Elvis operators for defaults
- [ ] Null checks before usage

### Resource Management
- [ ] Coroutines properly scoped
- [ ] Jobs cancelled in onCleared()
- [ ] Flows collected in viewModelScope
- [ ] No leaked contexts
- [ ] Battery monitor scope managed

### Error Handling
- [ ] Try-catch around risky operations
- [ ] Graceful degradation
- [ ] Error messages shown to user
- [ ] Logs for debugging
- [ ] No crashes on invalid input

### Performance
- [ ] No unnecessary recompositions
- [ ] Efficient StateFlow usage
- [ ] Lazy loading where applicable
- [ ] No blocking UI thread
- [ ] Battery monitoring not excessive

## Accessibility
- [ ] Content descriptions on icons
- [ ] Semantic labels on controls
- [ ] Talkback support
- [ ] Sufficient touch targets (48dp min)
- [ ] Color contrast ratios met

## Known Limitations
- Drag-to-reorder in queue (visual only, not functional)
- Level matching in A/B mode (basic, not normalized)
- Battery estimates (rough calculation)
- Signal path source format (inferred from filename)
