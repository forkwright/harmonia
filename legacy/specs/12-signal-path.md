# Spec 12: signal path visualization

**Status:** Draft
**Priority:** High
**Depends On:** Spec 08 (quality color variables)

## Goal

Redesign the signal path display from a flat chain of monochrome chips into a Roon-inspired quality visualization that tells the user, at a glance, what's happening to their audio and whether quality is being preserved, enhanced, or degraded. This is the feature that builds trust with audiophile users. It says: "we care about your signal as much as you do."

## Design philosophy

The signal path is not a technical diagram; it's a health indicator. Like a pulse oximeter: green means you're fine, yellow means pay attention, red means something's wrong. The user who doesn't care about audio quality should be able to ignore it. The user who does should be able to read the entire processing chain at a glance and know exactly where their signal is being altered.

Roon's insight: **color encodes quality, not processing stage.** A lossless source decoded losslessly is blue. The same source with EQ applied turns purple (enhanced). Resampling to a lower rate turns yellow (degraded). The colors flow through the chain, telling a story.

## Phases

### Phase 1: quality-coded signal chain

Redesign `SignalPath.tsx` with quality-aware coloring.

**Quality tiers:**

| Tier | Color | Variable | Meaning | Example |
|------|-------|----------|---------|---------|
| Enhanced | Purple | `--quality-enhanced` | Source quality maintained + processing added | Hi-res + EQ active |
| Lossless | Blue | `--quality-lossless` | Bit-perfect signal, no processing | FLAC → Decode → Output at native rate |
| High Quality | Green | `--quality-high` | Minor processing, no perceptible loss | Lossless + volume adjustment |
| Standard | Amber | `--quality-standard` | Lossy source or significant processing | MP3 source, or resampling |
| Low Quality | Red | `--quality-low` | Degraded signal path | Lossy + browser resampling + processing |

**Quality propagation rules:**

```
Source quality = f(format, sampleRate, bitDepth)
  FLAC/WAV/ALAC + >48kHz/24bit → Enhanced (purple)
  FLAC/WAV/ALAC at 44.1/48kHz 16bit → Lossless (blue)
  AAC/OGG ≥256kbps → High Quality (green)
  MP3/AAC <256kbps → Standard (amber)

Processing impact:
  EQ active → max(source_tier, "Enhanced") — EQ is intentional enhancement
  Compressor active → tier stays or drops to High Quality
  Volume only → no tier change (gain is lossless in float domain)
  Browser resampling → drops one tier (unavoidable quality loss)
  
Output quality = min(source_quality_after_processing, output_capability)
```

- [ ] Define quality tier calculation function from track metadata + pipeline state
- [ ] Color each node chip based on quality tier at that point in the chain
- [ ] Arrow connectors inherit the color of the downstream tier (shows quality flow)
- [ ] Overall quality badge: single colored dot/bar summarizing end-to-end quality
- [ ] Tooltip on each node: brief explanation of why it's that color

### Phase 2: improved node detail

Each node in the path should communicate more than a label.

```
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│  ■ FLAC 96kHz/24bit  ──▶  ■ Decode  ──▶  ■ EQ (10-band)       │
│    Enhanced                  Lossless       Enhanced             │
│                                                                  │
│  ──▶  ■ Volume (74%)  ──▶  ■ Web Audio 48kHz                   │
│         Lossless              Standard ⚠                        │
│                                                                  │
│  Overall: Standard — browser resamples 96kHz → 48kHz            │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

- [ ] Two-line node chip: top line = label + detail, bottom line = quality tier name
- [ ] Warning indicator (⚠) on nodes that cause quality loss
- [ ] Source node shows full format detail: `FLAC 96kHz/24bit` not just `FLAC`
- [ ] EQ node shows band count or profile name when active
- [ ] Output node shows actual device sample rate (from AudioContext)
- [ ] When browser resampling occurs (source rate ≠ output rate), highlight with amber/red

### Phase 3: interactive signal path

- [ ] Click a node to expand detail panel below the path:
  - Source: full file metadata (format, bitrate, sample rate, bit depth, channels, file size)
  - EQ: current band settings visualization (mini frequency response curve)
  - Compressor: threshold, ratio, knee, current gain reduction
  - Output: AudioContext state, latency, buffer size
- [ ] Expanded detail panel has the same quality-tier color border
- [ ] Only one node expanded at a time (accordion behavior)

### Phase 4: signal path in mini-player

The full signal path lives on the Player page. The mini-player should carry a condensed version.

- [ ] Single quality dot (colored by overall tier) in the mini-player bar
- [ ] Tooltip on the dot shows the abbreviated path: `FLAC 96/24 → EQ → 48kHz (Standard)`
- [ ] Dot position: between the track info and the play button, subtle but present

### Phase 5: pipeline transparency

Expose the `getPipelineState()` data more completely.

- [ ] Input → Processing → Output pipeline as structured data
- [ ] Latency measurement displayed: `3.2ms pipeline latency`
- [ ] Buffer health indicator (if buffer is running low during playback)
- [ ] Sample rate conversion indicator: when AudioContext rate ≠ source rate, show explicitly
- [ ] Bit-depth truncation indicator: when source is 24-bit but output is 16-bit (common in browsers)

## Technical notes

### Quality calculation

The quality tier is computed per-node, not globally. Each node receives the incoming quality tier from its predecessor and may preserve, improve, or degrade it.

```typescript
type QualityTier = 'enhanced' | 'lossless' | 'high' | 'standard' | 'low'

function getSourceTier(track: Track): QualityTier {
  const lossless = ['flac', 'wav', 'alac', 'aiff'].includes(track.format?.toLowerCase())
  const hiRes = track.sampleRate > 48000 || track.bitDepth > 16
  
  if (lossless && hiRes) return 'enhanced'
  if (lossless) return 'lossless'
  if (track.bitrate >= 256000) return 'high'
  return 'standard'
}

function applyProcessingImpact(
  incomingTier: QualityTier,
  node: { type: string; active: boolean; params?: Record<string, unknown> }
): QualityTier {
  if (!node.active) return incomingTier  // bypassed = no change
  
  switch (node.type) {
    case 'eq':
      // EQ is intentional enhancement — elevate to at least 'enhanced' if source is good
      return tierMax(incomingTier, 'high') // EQ on lossless+ → enhanced
    case 'compressor':
      return incomingTier  // compressor doesn't inherently degrade
    case 'volume':
      return incomingTier  // gain in float domain is lossless
    case 'resample':
      return tierMin(incomingTier, 'standard')  // resampling always degrades
    default:
      return incomingTier
  }
}
```

### Data sources

- **Track metadata:** `currentTrack` from `playerStore`: format, sampleRate, bitDepth, channels
- **Processing state:** `getEqualizer()`, `getCompressor()` from `useWebAudioPlayer`: enabled, params
- **Output state:** `AudioContext.sampleRate`, `AudioContext.baseLatency`: actual device capabilities
- **Pipeline state:** `getPipelineState()`: full pipeline snapshot

## Dependencies

- Spec 09 Phase 1 for quality color CSS variables
- Existing `SignalPath.tsx` component (rewrite, not new)
- Existing `useWebAudioPlayer` hook provides all necessary pipeline data

## Notes

- Roon charges $7/month. The signal path visualization is cited by users as the single feature that justifies the price. Implementing it well in an open-source player is high-value positioning.
- The "Browser Resampling" warning is always present on web. This is honest, not pessimistic. On Tauri desktop (Spec 02), the signal path could show a fully blue/purple chain with native output; that's the upgrade incentive.
- Don't animate the signal path continuously. It's a status display, not a visualization. Update when the track changes or processing state changes.
- Color-blind consideration: the quality tiers should also have text labels ("Enhanced", "Lossless", etc.) and distinct brightness levels, not just hue. Purple is darkest, blue is medium, green is bright, amber is warm, red is vivid.
- The EQ enhancement philosophy: applying EQ to correct for headphone response (AutoEQ) is enhancement, not degradation. The signal is being improved toward the mastering engineer's intent. This is Roon's view and it's correct.
