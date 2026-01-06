---
name: Signal Path Source Format Detection
about: Implement file metadata introspection for signal path
title: '[Android] Implement file metadata introspection for signal path'
labels: 'enhancement, android, audio, s'
assignees: ''
---

## Context

Current signal path visualization infers source format from filename (e.g., `.flac` → "FLAC 16/44.1"). This is unreliable and doesn't detect actual bit depth or sample rate. Need to introspect file metadata using native decoder or MediaMetadataRetriever.

**Current behavior:**
- Signal path shows "FLAC 16/44.1" based on filename extension
- Actual format unknown (could be 24/96 FLAC)

**Desired behavior:**
- Signal path shows actual format from file metadata
- "FLAC 24/96", "DSD64", "PCM 32/192" based on introspection

## Scope

### Format Detection Methods

#### Option 1: MediaMetadataRetriever (Simplest)
```kotlin
val retriever = MediaMetadataRetriever()
retriever.setDataSource(filePath)
val sampleRate = retriever.extractMetadata(METADATA_KEY_SAMPLERATE)
val bitrate = retriever.extractMetadata(METADATA_KEY_BITRATE)
```

**Pros**: Built-in Android API, no dependencies
**Cons**: May not expose bit depth, limited format support

#### Option 2: Native Decoder (Most Accurate)
- Use existing libFLAC/DSD decoder
- Read file header, extract format metadata
- Parse bit depth, sample rate, channel count

**Pros**: Most accurate, already have decoders
**Cons**: More complex, format-specific parsing

#### Option 3: FFmpeg/MediaInfo (Heavyweight)
- Integrate FFmpeg or MediaInfo library
- Full format detection for all codecs

**Pros**: Comprehensive format support
**Cons**: Large dependency, overkill for MVP

**Recommendation**: Start with Option 1 (MediaMetadataRetriever), upgrade to Option 2 if insufficient.

### Implementation

1. **Add format detection utility**
   - Create `AudioFormatDetector.kt`
   - Method: `detectFormat(filePath: String): AudioFormat`
   - Return: `data class AudioFormat(codec, sampleRate, bitDepth, channels)`

2. **Update SignalPathViewModel**
   - Replace filename inference with `detectFormat()`
   - Update `sourceFormat` StateFlow with actual metadata

3. **Handle edge cases**
   - File not found: Show "Unknown"
   - Unsupported format: Show codec name only
   - DSD files: Detect DSD64/128/256 rate

4. **Performance optimization**
   - Cache format metadata in Room database
   - Avoid re-reading on every playback

## Acceptance Criteria

- [ ] Signal path shows actual format from file metadata
- [ ] Bit depth detected correctly (16/24/32-bit)
- [ ] Sample rate detected correctly (44.1/48/96/192 kHz)
- [ ] Works for FLAC, ALAC, WAV, DSD formats
- [ ] Format metadata cached to avoid repeated file reads
- [ ] Graceful fallback for unsupported formats
- [ ] Performance: Format detection < 50ms per file

## Dependencies

- MediaMetadataRetriever (built-in Android API)
- Or libFLAC/DSD native decoders (already integrated)
- Room database for format cache

## Out of Scope

- Video format detection (audio-only app)
- Streaming format detection (local files only for MVP)
- Codec-specific details (focus on sample rate, bit depth, channels)
- Dynamic format changes during playback (assumes static file)

## Testing

### Test Files Needed
- FLAC 16/44.1
- FLAC 24/96
- FLAC 24/192
- DSD64, DSD128
- ALAC (Apple Lossless)
- WAV 16/44.1, 24/96

### Validation
- Compare detected format with known file specs
- Verify signal path UI displays correct values
- Test cache hit rate (should be >95% for repeated plays)

## Platform(s)

Android

## Size Estimate

**s** (1-4 hours)

**Breakdown:**
- AudioFormatDetector utility: 1-2 hours
- SignalPathViewModel integration: 1 hour
- Testing and caching: 1 hour
