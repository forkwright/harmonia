# Spec 07: Competitive Analysis

**Status:** Active
**Priority:** Medium

## Goal

Map the competitive landscape for unified self-hosted media players to validate Akroasis's positioning, identify audio feature gaps, and surface patterns worth adopting. The central finding: no self-hosted player unifies music, audiobooks, and podcasts with audiophile-grade audio. That gap is real and Akroasis owns it — but individual competitors lead on specific features that matter.

## Landscape Overview

### Subsonic-Compatible Players (Direct Market)

| Player | Platform | Stars/Installs | Servers | Audio Quality | Audiobook | Podcast | Open Source |
|--------|----------|---------------|---------|---------------|-----------|---------|-------------|
| **Symfonium** | Android | 450K+ installs | 12+ (Plex, Jellyfin, Subsonic, cloud) | 256-band EQ, ReplayGain, DSD. No bit-perfect, no crossfeed | Partial (queue-based) | No | No |
| **Feishin** | Desktop/Web | 7.2K | Navidrome, Jellyfin | Gapless via MPV. No EQ, no bit-perfect | No | No | GPL-3.0 |
| **Supersonic** | Desktop | 2K | Subsonic, Jellyfin | 15-band EQ, ReplayGain, waveform seekbar | No | No | Yes |
| **Finamp** | Android/iOS | 3.7K | Jellyfin only | Gapless, ReplayGain. No EQ | No (fork exists) | No | Yes |
| **Tempo** | Android | 2.1K | Subsonic/Navidrome | Gapless, Last.fm scrobbling | No | Yes | GPL-3.0 |
| **Amperfy** | iOS/macOS | — | Ampache, Subsonic | EQ, ReplayGain, CarPlay/Siri | No | Yes | Yes |
| **play:Sub** | iOS | — | Subsonic ecosystem | 10-band EQ, ReplayGain, crossfade | Bookmarks | Yes | No |
| **Akroasis** | Web + Android | — | Mouseion | 5-band EQ, crossfeed, bit-perfect, ReplayGain, gapless | **Yes (chapters)** | **Planned** | **Yes** |

### Audiophile Players

| Player | Platform | Price | Bit-Perfect | EQ | Crossfeed | DSD | Key Feature |
|--------|----------|-------|-------------|-----|-----------|-----|-------------|
| **Neutron** | Android/iOS/Win | $6-8 | Yes | 4-60 band parametric | Yes (Ambiophonic) | Native DSD512 | 5,000 AutoEQ profiles, surround, DAP profiles |
| **Poweramp** | Android | $3.99 | Partial | 64-band parametric | No | Yes | 96M downloads, DVC, per-device EQ |
| **UAPP** | Android | $7.99 | Yes (custom USB driver) | — | — | Native + MQA | Bypasses Android mixer entirely |
| **Roon** | All | $7/mo or $500 | Yes | Parametric | Yes | DSD512 | Signal path viz, convolution, room correction |
| **Audirvana** | Desktop | $7/mo or $120 | Yes | — | Yes | Yes | Exclusive device access, convolution |
| **Strawberry** | Desktop | Free | Yes (Linux) | Yes | — | — | EBU R128, GStreamer, open source |
| **Akroasis** | Web + Android | Free | **Yes** | **5-band** | **Yes** | No | Rust core, self-hosted, open source |

### Audiobook & Podcast Players

| Player | Category | Stars | Platform | Key Features |
|--------|----------|-------|----------|-------------|
| **Audiobookshelf** | Audiobook + Podcast server | 11.8K | Web + mobile | Dominant. Node.js, real-time sync, Audnexus chapters, offline apps, multi-user |
| **PinePods** | Podcast server | 800 | Web + mobile | Rust backend, multi-user, gPodder compat, built-in search |
| **AntennaPod** | Podcast client | 7.7K | Android | Reference client, gPodder sync, no server |
| **BookPlayer** | Audiobook client | — | iOS | Local-only, CarPlay, Apple Watch |
| **Voice** | Audiobook client | — | Android | Local-only, minimalist, Material Design |

### Unified Players (Music + Audiobooks + Podcasts)

**The space is empty.** No self-hosted project unifies all three with quality audio:

- **Spotify** — commercial proof unification works, but lossy streaming, not self-hosted
- **Plex ecosystem** — fragments across Plexamp (music) + Prologue (audiobooks) + podcast feature. Requires Plex Pass ($7/mo or $250)
- **Jellyfin** — serves all types but no single client handles them well. Finamp for music, no good audiobook client
- **Symfonium** — closest: music + audiobook queues, but no podcasts, no bit-perfect, closed source, Android only
- **play:Sub** — music + bookmarks + podcasts on iOS, but no real chapter support, closed source, iOS only

## Competitor Deep Dives

### Symfonium — The Mobile Benchmark

The most polished self-hosted music client. Connects to 12+ server types simultaneously (Plex, Jellyfin, Subsonic, cloud storage). 450K+ installs.

**Where it leads:**
- 256-band EQ with AutoEQ integration (4,200+ headphone profiles)
- Smart Fades (music-aware crossfades)
- Casting: Chromecast, UPnP/DLNA, Kodi, Sonos with full group control
- Wear OS companion app
- Multi-server aggregation (browse all servers as unified library)
- DSD playback (dff/dsf)

**Where Akroasis leads:**
- Bit-perfect playback (Symfonium developer confirmed they don't do bit-perfect)
- Crossfeed DSP
- Audiobook chapter navigation (Symfonium uses generic queues)
- Open source
- Web client
- Rust audio core (Symfonium uses platform audio APIs)

**What to learn:** AutoEQ headphone profiles are table stakes for audiophile credibility. Server aggregation is compelling if Mouseion ever supports multiple instances. Casting support (especially Chromecast) is highly requested.

---

### Audiobookshelf — The Audiobook Standard

11.8K stars. Dominant self-hosted audiobook + podcast manager. Node.js/Express + SQLite + Socket.IO.

**Where it leads:**
- Real-time WebSocket progress sync across devices
- Audnexus API for chapter metadata
- Offline mobile apps with background sync
- Multi-user with per-user progress
- Podcast management (feed parsing, auto-download, auto-cleanup)
- Collections, series grouping, library stats
- 9 specialized backend managers (modular architecture)

**Where Akroasis leads:**
- Audio quality (bit-perfect, EQ, crossfeed — Audiobookshelf has basic playback)
- Music support (Audiobookshelf explicitly excludes music)
- Rust audio core vs JavaScript audio
- A/B testing with level matching

**What to learn:** The real-time WebSocket sync model for cross-device progress. Audnexus integration for chapter metadata. The "audiobook + podcast in one server" proves demand for unification. Audiobookshelf's 11.8K stars dwarf every music client — audiobook users are underserved and passionate.

---

### Neutron — The Audio Quality Ceiling

The most feature-complete audiophile player. Closed source, paid.

**Where it leads:**
- 4-60 band parametric EQ (vs Akroasis's 5-band)
- Ambiophonic R.A.C.E. surround processing
- Native DSD to DSD512, SACD ISO support
- 5,000+ AutoEQ headphone presets
- AI-assisted EQ generation
- DAP-specific output profiles (iBasso, Cayin, Astell&Kern, FiiO, etc.)
- 32/64-bit internal processing up to 1.536 MHz
- Compressor/limiter for dynamic range
- Network renderer output (UPnP speakers with DSP applied)

**Where Akroasis leads:**
- Self-hosted backend integration (Neutron is local/network files only)
- Audiobook support
- Open source
- Web + mobile cross-platform

**What to learn:** Parametric EQ is the audiophile standard. DAP profiles are high-value for the portable audio market. Signal path and processing chain transparency builds trust. Compressor/limiter is underrated for audiobook and podcast listening.

---

### Feishin — The Desktop UI Reference

7.2K stars. Best-looking desktop Subsonic client. Electron + MPV backend.

**Where it leads:**
- Polished modern UI (the visual bar for self-hosted music)
- Smart playlist editor (Navidrome integration)
- Synchronized lyrics
- MPV backend gives near-native audio quality on desktop

**Where Akroasis leads:**
- Audio DSP (no EQ in Feishin)
- Audiobook support
- Offline support
- Mobile client
- Not Electron (~150MB+ RAM overhead)

**What to learn:** Synchronized lyrics are expected in 2026. The Navidrome smart playlist editor UX is worth studying. MPV as audio backend on desktop (via Tauri integration) gives format support and gapless for free.

---

### Roon — The Premium Reference

$7/month or $500 lifetime. The standard audiophiles measure against.

**Where it leads:**
- Signal Path visualization (real-time display of all processing stages)
- Convolution engine for room correction
- Parametric + procedural EQ
- Music discovery and metadata richness (MusicBrainz, AllMusic, credits browsing)
- Multi-room with per-zone DSP

**Where Akroasis leads:**
- Free and open source
- Self-hosted (Roon requires their cloud for metadata)
- Audiobook + podcast support
- Mobile-first design

**What to learn:** Signal path transparency is the single most trust-building feature for audiophiles. Convolution engine is the desktop audiophile dream. Credits/liner notes browsing enriches the listening experience.

---

### Desktop Landscape (Tauri Opportunity)

| Player | Stack | Stars | Audio Backend | Status |
|--------|-------|-------|---------------|--------|
| **musicat** | Tauri + Svelte | 858 | — | v0.15, local files only |
| **Nuclear** | Electron → Tauri migration | 17K | — | Rewrite in progress, not shipping |
| **Harmonoid** | Flutter/Dart | 4.4K | libmpv | ~150MB RAM, cross-platform |
| **Tauon** | Python + GTK | — | GStreamer/BASS | Linux-focused, feature-rich |

The Tauri music player space is nascent. No Tauri player supports streaming from a server, audiobooks, or podcasts. Akroasis's Tauri desktop app would be first-to-market for self-hosted unified audio.

**PipeWire/ALSA integration:** The Rust ecosystem offers rodio + cpal + symphonia for native audio output. Alternatively, wrapping libmpv (like Feishin/Harmonoid) gives broader format support and proven gapless. The Rust audio core already exists — desktop FFI bindings are the remaining bridge.

## Feature Matrix

| Capability | Symfonium | Feishin | Audiobookshelf | Neutron | Roon | **Akroasis** |
|------------|-----------|---------|----------------|---------|------|-------------|
| Music | Yes | Yes | No | Yes | Yes | **Yes** |
| Audiobooks (chapters) | Partial | No | **Yes** | No | No | **Yes** |
| Podcasts | No | No | **Yes** | No | No | **Planned** |
| Bit-perfect | No | No | No | **Yes** | **Yes** | **Yes** |
| EQ bands | 256 | 0 | 0 | 60 param | Param | **5** |
| AutoEQ profiles | 4,200 | 0 | 0 | 5,000 | 0 | **0** |
| Crossfeed | No | No | No | **Yes** | **Yes** | **Yes** |
| ReplayGain | Yes | No | No | Yes | Yes | **Yes** |
| Gapless | Yes | Yes (MPV) | No | Yes | Yes | **Yes** |
| Signal path viz | No | No | No | No | **Yes** | **Partial** |
| DSD | Yes | No | No | **DSD512** | **DSD512** | No |
| Casting | Chromecast+DLNA+Sonos | No | No | DLNA | Multi-room | **No** |
| Scrobbling | Server | Server | No | No | No | **Last.fm + LB** |
| Offline sync | Yes | No | **Yes** | Local | No | **Planned** |
| Lyrics | Yes | **Yes** | No | No | Yes | No |
| Cross-platform | Android | Desktop/Web | Web + mobile | Android/iOS/Win | All | **Web + Android** |
| Open source | No | Yes | Yes | No | No | **Yes** |
| Self-hosted backend | Multi-server | Navidrome/Jellyfin | Built-in | None | Roon Core | **Mouseion** |

## Strategic Findings

### Akroasis's Moat

**Unified self-hosted audio player does not exist.** No project combines music + audiobooks + podcasts with audiophile audio quality and a self-hosted backend. Users currently run 2-3 separate apps (Navidrome client + Audiobookshelf + AntennaPod). Akroasis is the only project attempting to collapse this stack.

**Bit-perfect + crossfeed + open source is unique.** The audiophile players (Neutron, UAPP, Poweramp, Roon) are all closed source. The open source players (Feishin, Finamp, Strawberry) lack audiophile DSP. Akroasis is the only open source player with both bit-perfect and crossfeed.

**Rust audio core is a structural advantage.** Every competitor uses platform audio APIs, MPV wrappers, or GStreamer. A shared Rust core compiled to JNI (Android) + FFI (desktop) + WASM (web, future) gives performance and consistency no competitor matches architecturally.

### Gaps to Close

1. **EQ depth** — 5-band is behind market. Symfonium: 256, Neutron: 60 parametric, Poweramp: 64 parametric, Supersonic: 15. Minimum 10-band, ideally parametric mode.
2. **AutoEQ headphone profiles** — Symfonium ships 4,200, Neutron ships 5,000. This is expected by audiophiles and straightforward to integrate (AutoEQ database is open source).
3. **Casting** — Chromecast and UPnP/DLNA output are expected. Symfonium even does Sonos group control.
4. **Synchronized lyrics** — Feishin, Symfonium, and most modern players support them. Standard feature in 2026.
5. **Desktop app** — Tauri shell exists but audio isn't wired. The desktop space is wide open for a self-hosted unified player.
6. **DSD support** — Matters for audiophile credibility. Neutron, Poweramp, UAPP, Roon all support it. Server-side transcoding may suffice if Mouseion handles conversion.

### Patterns Worth Adopting

| Pattern | Source | Value |
|---------|--------|-------|
| Per-output EQ presets | Poweramp | Switch headphones → EQ follows automatically |
| Signal path display | Roon | Real-time visualization of all processing stages |
| AutoEQ integration | Symfonium, Neutron | Open source headphone correction database |
| Offline scrobble queue | play:Sub | Scrobble when back online after offline listening |
| Smart/dynamic playlists | Symfonium, Feishin | Server-side filter rules, auto-updating |
| EBU R128 normalization | Strawberry | More modern than ReplayGain alone |
| DVC (Direct Volume Control) | Poweramp | Distortion-free EQ at high volume |
| Waveform seekbar | Supersonic | Visual feedback for audio position |
| gPodder protocol | AntennaPod, PinePods | Standard for podcast sync interop |
| WebSocket progress sync | Audiobookshelf | Real-time cross-device position updates |

## Phases

### Phase 1: Close critical audio gaps
- [ ] Expand EQ to 10-band parametric mode (both Android and web)
- [ ] Integrate AutoEQ headphone profiles database (open source, ~5K profiles)
- [ ] Add per-output EQ preset switching (detect output device change → load saved EQ)
- [ ] Add signal path display showing all active processing stages

### Phase 2: Content type completion
- [ ] Podcast support: feed parsing, episode tracking, auto-cleanup (Spec 01 Phase 2)
- [ ] Synchronized lyrics display (fetch from LRCLIB or embedded metadata)
- [ ] Sleep timer with end-of-chapter mode for audiobooks
- [ ] Offline scrobble queue (buffer scrobbles when offline, flush on reconnect)

### Phase 3: Platform expansion
- [ ] Desktop Tauri app with PipeWire/ALSA audio output via Rust core FFI
- [ ] Chromecast output support (web: Cast SDK, Android: Cast framework)
- [ ] UPnP/DLNA renderer output
- [ ] Android Auto full media browser (beyond current shell)

### Phase 4: Audiophile differentiation
- [ ] DSD passthrough to USB DAC (Android — extend Rust core)
- [ ] EBU R128 loudness normalization alongside ReplayGain
- [ ] Waveform seekbar visualization
- [ ] Compressor/limiter DSP for podcast and audiobook listening comfort

## Dependencies

- Podcast support blocked on Mouseion podcast API (Spec 01 in Mouseion)
- Lyrics depend on LRCLIB API or Mouseion metadata enrichment
- Chromecast requires Google Cast SDK (web) and Cast framework (Android)
- Desktop audio output requires Rust core FFI bindings (partially exists)
- AutoEQ database is MIT-licensed at github.com/jaakkopasanen/AutoEq

## Notes

- Research conducted Feb 2026. Star counts and market positions will shift.
- Audiobookshelf at 11.8K stars vs Navidrome at 19.3K stars shows audiobook users are nearly as large a market as music users. Akroasis bridging both is high leverage.
- Symfonium's 450K installs at $5/app proves self-hosted music players have real commercial demand.
- Poweramp's 96M downloads proves mobile audiophile is not niche.
- The Tauri desktop space is nascent — first-mover advantage is real if Spec 02 ships.
- No competitor has a Rust audio core. This is a structural moat that compounds as more DSP features are added (one implementation, all platforms).
- gPodder protocol is the de facto standard for podcast sync — worth supporting for interop even with Mouseion as primary backend.
