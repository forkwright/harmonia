pub mod prebuffer;

/// Transition mode between two tracks in gapless playback.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransitionMode {
    /// True gapless: the next track's pre-buffer is spliced immediately after the last sample
    /// of the current track (after gapless trimming).
    Gapless,
    /// Cross-fade: the tail of the current track and the head of the next are mixed over
    /// `duration_ms` milliseconds.
    Crossfade { duration_ms: u32 },
    /// A hard gap (silence) is inserted between tracks.
    Gap { duration_ms: u32 },
}

/// Coordinates pre-buffering and scheduling of the next track.
///
/// The scheduler watches the current track's remaining sample count and begins
/// decoding the next track when the ring buffer drops below `prebuffer_threshold`.
pub struct GaplessScheduler {
    _private: (),
}

impl GaplessScheduler {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for GaplessScheduler {
    fn default() -> Self {
        Self::new()
    }
}
