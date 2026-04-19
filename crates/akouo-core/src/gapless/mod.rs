pub mod prebuffer;

use std::collections::VecDeque;
use std::f64::consts::FRAC_PI_2;

use tracing::instrument;

use crate::decode::{GaplessInfo, metadata::TrackMetadata};
use crate::gapless::prebuffer::PreBuffer;

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

impl Default for TransitionMode {
    fn default() -> Self {
        TransitionMode::Crossfade { duration_ms: 3000 }
    }
}

/// Which end of the frame buffer to trim encoder delay FROM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimPosition {
    /// Trim encoder priming samples FROM the start of the track.
    Start,
    /// Trim encoder padding samples FROM the end of the track.
    End,
}

/// Trims encoder delay or padding samples FROM a frame buffer.
///
/// `encoder_delay` and `encoder_padding` in `GaplessInfo` are in per-channel samples.
/// This function accounts for interleaved layout by multiplying by `channels`.
///
/// A zero delay or padding value is a no-op.
#[instrument(skip(frames, gapless_info))]
pub fn trim_codec_delay(
    frames: &mut VecDeque<Box<[f64]>>,
    gapless_info: &GaplessInfo,
    position: TrimPosition,
    channels: u16,
) {
    let samples_to_trim = match position {
        TrimPosition::Start => {
            usize::try_from(gapless_info.encoder_delay).unwrap_or_default() * usize::from(channels)
        }
        TrimPosition::End => {
            usize::try_from(gapless_info.encoder_padding).unwrap_or_default()
                * usize::from(channels)
        }
    };

    if samples_to_trim == 0 {
        return;
    }

    match position {
        TrimPosition::Start => trim_from_start(frames, samples_to_trim),
        TrimPosition::End => trim_from_end(frames, samples_to_trim),
    }
}

fn trim_from_start(frames: &mut VecDeque<Box<[f64]>>, mut remaining: usize) {
    while remaining > 0 {
        let frame_len = match frames.front() {
            Some(f) => f.len(),
            None => break,
        };
        if frame_len <= remaining {
            remaining -= frame_len;
            frames.pop_front();
        } else {
            let frame = frames
                .pop_front()
                .unwrap_or_else(|| unreachable!("frames.front() returned Some above"));
            let trimmed = frame[remaining..].to_vec().into_boxed_slice();
            frames.push_front(trimmed);
            remaining = 0;
        }
    }
}

fn trim_from_end(frames: &mut VecDeque<Box<[f64]>>, mut remaining: usize) {
    while remaining > 0 {
        let frame_len = match frames.back() {
            Some(f) => f.len(),
            None => break,
        };
        if frame_len <= remaining {
            remaining -= frame_len;
            frames.pop_back();
        } else {
            let frame = frames
                .pop_back()
                .unwrap_or_else(|| unreachable!("frames.back() returned Some above"));
            let keep = frame.len() - remaining;
            let trimmed = frame[..keep].to_vec().into_boxed_slice();
            frames.push_back(trimmed);
            remaining = 0;
        }
    }
}

/// Produces an equal-power crossfade blend of `outgoing` and `incoming` at `position`.
///
/// `position` is in `[0.0, 1.0]`: `0.0` yields only the outgoing signal, `1.0` only
/// the incoming signal. The equal-power curve (`cos`/`sin`) preserves perceived loudness
/// across the blend window.
///
/// `outgoing` and `incoming` must have the same length. If they differ in debug builds
/// this panics; in release builds the shorter length is used.
pub fn crossfade_frame(outgoing: &[f64], incoming: &[f64], position: f64) -> Box<[f64]> {
    debug_assert_eq!(
        outgoing.len(),
        incoming.len(),
        "crossfade_frame: frame length mismatch"
    );

    let angle = position.clamp(0.0, 1.0) * FRAC_PI_2;
    let out_gain = angle.cos();
    let in_gain = angle.sin();

    outgoing
        .iter()
        .zip(incoming.iter())
        .map(|(&o, &i)| o * out_gain + i * in_gain)
        .collect::<Vec<f64>>()
        .into_boxed_slice()
}

/// Detects whether two consecutive tracks belong to the same album.
pub struct AlbumDetector;

impl AlbumDetector {
    /// Returns `true` when `current` and `next` share the same album and artist.
    ///
    /// Comparison is case-insensitive ASCII. If either album or artist is missing FROM
    /// either track, returns `false`.
    pub fn is_same_album(current: &TrackMetadata, next: &TrackMetadata) -> bool {
        let same_album = match (&current.album, &next.album) {
            (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
            _ => false,
        };
        let same_artist = match (&current.artist, &next.artist) {
            (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
            _ => false,
        };
        same_album && same_artist
    }

    /// Selects the appropriate transition mode for moving FROM `current` to `next`.
    ///
    /// Returns `Gapless` when both tracks are on the same album. Falls back to
    /// `TransitionMode::default()` otherwise.
    pub fn should_gapless(current: &TrackMetadata, next: &TrackMetadata) -> TransitionMode {
        if Self::is_same_album(current, next) {
            TransitionMode::Gapless
        } else {
            TransitionMode::default()
        }
    }
}

/// The overlap region WHERE the ending track's tail meets the starting track's head.
///
/// For a gapless splice this is empty; for a crossfade it holds both sides of the
/// window so `crossfade_frame` can blend them frame-by-frame.
pub struct CarryBuffer {
    /// Remaining samples FROM the ending track.
    pub outgoing: VecDeque<Box<[f64]>>,
    /// First samples FROM the starting track.
    pub incoming: VecDeque<Box<[f64]>>,
    /// Pre-computed equal-power fade positions, one per interleaved frame.
    pub fade_curve: Option<Box<[f64]>>,
}

impl CarryBuffer {
    pub fn new() -> Self {
        Self {
            outgoing: VecDeque::new(),
            incoming: VecDeque::new(),
            fade_curve: None,
        }
    }

    /// Creates a carry buffer pre-loaded with the fade curve for a crossfade of
    /// `duration_samples` per-channel samples at the given `sample_rate`.
    pub fn with_crossfade(duration_ms: u32, sample_rate: u32) -> Self {
        let duration_samples = (f64::from(duration_ms) / 1000.0 * f64::from(sample_rate)) as usize;
        let fade_curve = (0..duration_samples)
            .map(|i| i as f64 / duration_samples.max(1) as f64)
            .collect::<Vec<f64>>()
            .into_boxed_slice();
        Self {
            outgoing: VecDeque::new(),
            incoming: VecDeque::new(),
            fade_curve: Some(fade_curve),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.outgoing.is_empty() && self.incoming.is_empty()
    }
}

impl Default for CarryBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Coordinates pre-buffering and scheduling of the next track.
///
/// The scheduler watches the current track's remaining sample count and begins
/// decoding the next track when the ring buffer drops below `prebuffer_threshold`.
/// Transition logic (gapless splice, crossfade, or silence gap) is applied at
/// track boundaries by the engine (P1-10).
pub struct GaplessScheduler {
    pre_buffer: PreBuffer,
    carry_buffer: Option<CarryBuffer>,
    transition_mode: TransitionMode,
    prefetch_active: bool,
}

impl GaplessScheduler {
    pub fn new(pre_buffer: PreBuffer, transition_mode: TransitionMode) -> Self {
        Self {
            pre_buffer,
            carry_buffer: None,
            transition_mode,
            prefetch_active: false,
        }
    }

    /// Returns `true` when remaining playback time has dropped below the pre-buffer
    /// threshold and prefetching should begin.
    ///
    /// Once prefetching is active this returns `false` until `cancel_prefetch` is called.
    pub fn should_start_prefetch(&self, samples_remaining: u64, sample_rate: u32) -> bool {
        if self.prefetch_active {
            return false;
        }
        let threshold = (self.pre_buffer.threshold_secs() * f64::from(sample_rate)) as u64;
        samples_remaining <= threshold
    }

    /// Marks the pre-buffer task as active. Call after spawning the decode task.
    pub fn mark_prefetch_started(&mut self) {
        self.prefetch_active = true;
    }

    /// Returns `true` when a prefetch task is running.
    pub fn is_prefetch_active(&self) -> bool {
        self.prefetch_active
    }

    /// Cancels the running prefetch task and resets state. Call on stop or skip.
    #[instrument(skip(self))]
    pub fn cancel_prefetch(&mut self) {
        self.pre_buffer.cancel();
        self.pre_buffer.clear();
        self.prefetch_active = false;
        self.carry_buffer = None;
    }

    /// Selects the transition mode for moving FROM `current` to `next`.
    ///
    /// Uses album detection to pick gapless automatically; falls back to
    /// `self.transition_mode` only when overriding user preference takes precedence
    /// (i.e., the user has explicitly SET Gap or Crossfade in settings  -  the caller
    /// is responsible for deciding when to honour that over album detection).
    pub fn select_transition_mode(
        &self,
        current: &TrackMetadata,
        next: &TrackMetadata,
    ) -> TransitionMode {
        AlbumDetector::should_gapless(current, next)
    }

    /// Overrides the default transition mode (used when the user changes settings).
    pub fn set_transition_mode(&mut self, mode: TransitionMode) {
        self.transition_mode = mode;
    }

    pub fn transition_mode(&self) -> &TransitionMode {
        &self.transition_mode
    }

    pub fn pre_buffer(&self) -> &PreBuffer {
        &self.pre_buffer
    }

    pub fn pre_buffer_mut(&mut self) -> &mut PreBuffer {
        &mut self.pre_buffer
    }

    pub fn carry_buffer(&self) -> Option<&CarryBuffer> {
        self.carry_buffer.as_ref()
    }

    pub fn carry_buffer_mut(&mut self) -> Option<&mut CarryBuffer> {
        self.carry_buffer.as_mut()
    }

    pub fn set_carry_buffer(&mut self, buf: CarryBuffer) {
        self.carry_buffer = Some(buf);
    }

    pub fn take_carry_buffer(&mut self) -> Option<CarryBuffer> {
        self.carry_buffer.take()
    }
}

impl Default for GaplessScheduler {
    fn default() -> Self {
        Self::new(
            PreBuffer::new(10.0, 44100 * 10 * 2),
            TransitionMode::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::GaplessInfo;
    use crate::decode::metadata::TrackMetadata;

    // ── crossfade_frame ──────────────────────────────────────────────────────

    #[test]
    fn crossfade_position_zero_is_all_outgoing() {
        let out: Vec<f64> = vec![1.0, 0.5, -0.5, -1.0];
        let inc: Vec<f64> = vec![0.2, 0.3, 0.4, 0.5];
        let result = crossfade_frame(&out, &inc, 0.0);
        for (r, &o) in result.iter().zip(out.iter()) {
            assert!(
                (r - o).abs() < 1e-12,
                "at position 0 output should equal outgoing"
            );
        }
    }

    #[test]
    fn crossfade_position_one_is_all_incoming() {
        let out: Vec<f64> = vec![1.0, 0.5, -0.5, -1.0];
        let inc: Vec<f64> = vec![0.2, 0.3, 0.4, 0.5];
        let result = crossfade_frame(&out, &inc, 1.0);
        for (r, &i) in result.iter().zip(inc.iter()) {
            assert!(
                (r - i).abs() < 1e-12,
                "at position 1 output should equal incoming"
            );
        }
    }

    #[test]
    fn crossfade_position_half_equal_power_blend() {
        // At 0.5 both gains should be cos(π/4) = sin(π/4) ≈ 0.7071
        let out: Vec<f64> = vec![1.0, 1.0];
        let inc: Vec<f64> = vec![0.0, 0.0];
        let result = crossfade_frame(&out, &inc, 0.5);
        let expected_gain = (FRAC_PI_2 * 0.5_f64).cos();
        for &r in result.iter() {
            assert!((r - expected_gain).abs() < 1e-12);
        }
    }

    #[test]
    fn crossfade_energy_conservation() {
        // Equal-power property: out_gain² + in_gain² = 1 at any position
        let out: Vec<f64> = vec![1.0];
        let inc: Vec<f64> = vec![1.0];
        for i in 0..=100 {
            let pos = f64::from(i) / 100.0;
            let result = crossfade_frame(&out, &inc, pos);
            let angle = pos * FRAC_PI_2;
            // The result for unit inputs: cos(angle) + sin(angle)
            // Power of output signal = result² compared to ideal constant-power:
            // out_gain² + in_gain² = 1, so total power across both channels is preserved
            let out_gain = angle.cos();
            let in_gain = angle.sin();
            let power = out_gain * out_gain + in_gain * in_gain;
            assert!(
                (power - 1.0).abs() < 1e-12,
                "power conservation failed at pos={pos}"
            );
            // Verify the blended value matches formula
            let expected = out_gain * out.first().copied().unwrap_or_default()
                + in_gain * inc.first().copied().unwrap_or_default();
            assert!((result.first().copied().unwrap_or_default() - expected).abs() < 1e-12);
        }
    }

    // ── trim_codec_delay ─────────────────────────────────────────────────────

    fn make_frames(samples_per_frame: usize, num_frames: usize) -> VecDeque<Box<[f64]>> {
        let mut q = VecDeque::new();
        let mut counter = 0.0_f64;
        for _ in 0..num_frames {
            let frame: Box<[f64]> = (0..samples_per_frame)
                .map(|_| {
                    let v = counter;
                    counter += 1.0;
                    v
                })
                .collect::<Vec<f64>>()
                .into_boxed_slice();
            q.push_back(frame);
        }
        q
    }

    fn total_samples(frames: &VecDeque<Box<[f64]>>) -> usize {
        frames.iter().map(|f| f.len()).sum()
    }

    #[test]
    fn trim_mp3_encoder_delay_removes_start_samples() {
        // Simulate MP3 with 576 samples encoder delay, stereo (channels=2)
        let encoder_delay = 576_u32;
        let channels = 2_u16;
        // 4 frames of 1024 stereo samples = 4096 interleaved VALUES
        let mut frames = make_frames(1024, 4);
        let before = total_samples(&frames);

        let info = GaplessInfo {
            encoder_delay,
            encoder_padding: 0,
            total_samples: None,
        };
        trim_codec_delay(&mut frames, &info, TrimPosition::Start, channels);

        let after = total_samples(&frames);
        let removed = before - after;
        assert_eq!(
            removed,
            usize::try_from(encoder_delay).unwrap_or_default() * usize::from(channels)
        );
        // First remaining sample should be value 1152 (576 * 2)
        assert_eq!(
            frames[0][0],
            (usize::try_from(encoder_delay).unwrap_or_default() * usize::from(channels)) as f64
        );
    }

    #[test]
    fn trim_mp3_encoder_padding_removes_end_samples() {
        let encoder_padding = 576_u32;
        let channels = 2_u16;
        let mut frames = make_frames(1024, 4);
        let before = total_samples(&frames);

        let info = GaplessInfo {
            encoder_delay: 0,
            encoder_padding,
            total_samples: None,
        };
        trim_codec_delay(&mut frames, &info, TrimPosition::End, channels);

        let after = total_samples(&frames);
        let removed = before - after;
        assert_eq!(
            removed,
            usize::try_from(encoder_padding).unwrap_or_default() * usize::from(channels)
        );
    }

    #[test]
    fn trim_flac_zero_delay_no_change() {
        // FLAC returns None for GaplessInfo; if called with zero delay, nothing changes
        let channels = 2_u16;
        let mut frames = make_frames(512, 2);
        let before = total_samples(&frames);

        let info = GaplessInfo {
            encoder_delay: 0,
            encoder_padding: 0,
            total_samples: None,
        };
        trim_codec_delay(&mut frames, &info, TrimPosition::Start, channels);
        trim_codec_delay(&mut frames, &info, TrimPosition::End, channels);

        assert_eq!(total_samples(&frames), before);
    }

    #[test]
    fn trim_opus_preskip_removes_correct_samples() {
        // Opus standard pre-skip is 3840 samples (80ms at 48 kHz), channels=2
        let encoder_delay = 3840_u32;
        let channels = 2_u16;
        // 8 frames of 960 stereo samples (one Opus frame worth per frame)
        let mut frames = make_frames(960 * usize::from(channels), 8);
        let before = total_samples(&frames);

        let info = GaplessInfo {
            encoder_delay,
            encoder_padding: 0,
            total_samples: None,
        };
        trim_codec_delay(&mut frames, &info, TrimPosition::Start, channels);

        let after = total_samples(&frames);
        let removed = before - after;
        assert_eq!(
            removed,
            usize::try_from(encoder_delay).unwrap_or_default() * usize::from(channels)
        );
    }

    #[test]
    fn trim_delay_spanning_multiple_frames() {
        // Delay larger than a single frame  -  must consume multiple frames
        let encoder_delay = 1500_u32;
        let channels = 1_u16;
        let mut frames = make_frames(1024, 4); // 4 * 1024 = 4096 samples mono
        let before = total_samples(&frames);

        let info = GaplessInfo {
            encoder_delay,
            encoder_padding: 0,
            total_samples: None,
        };
        trim_codec_delay(&mut frames, &info, TrimPosition::Start, channels);

        let after = total_samples(&frames);
        assert_eq!(
            before - after,
            usize::try_from(encoder_delay).unwrap_or_default()
        );
        // After removing 1500 samples FROM 4*1024=4096, we have 2596 LEFT
        assert_eq!(after, 4096 - 1500);
    }

    // ── AlbumDetector ────────────────────────────────────────────────────────

    fn track(
        album: Option<&str>,
        artist: Option<&str>,
        track_number: Option<u32>,
    ) -> TrackMetadata {
        TrackMetadata {
            album: album.map(str::to_string),
            artist: artist.map(str::to_string),
            track_number,
            ..Default::default()
        }
    }

    #[test]
    fn same_album_consecutive_tracks_is_gapless() {
        let current = track(Some("Dark Side of the Moon"), Some("Pink Floyd"), Some(1));
        let next = track(Some("Dark Side of the Moon"), Some("Pink Floyd"), Some(2));
        assert!(AlbumDetector::is_same_album(&current, &next));
        assert_eq!(
            AlbumDetector::should_gapless(&current, &next),
            TransitionMode::Gapless
        );
    }

    #[test]
    fn same_album_nonconsecutive_tracks_is_still_gapless() {
        let current = track(Some("Dark Side of the Moon"), Some("Pink Floyd"), Some(1));
        let next = track(Some("Dark Side of the Moon"), Some("Pink Floyd"), Some(5));
        assert!(AlbumDetector::is_same_album(&current, &next));
        assert_eq!(
            AlbumDetector::should_gapless(&current, &next),
            TransitionMode::Gapless
        );
    }

    #[test]
    fn different_albums_returns_user_default() {
        let current = track(Some("Dark Side of the Moon"), Some("Pink Floyd"), Some(1));
        let next = track(Some("Wish You Were Here"), Some("Pink Floyd"), Some(1));
        assert!(!AlbumDetector::is_same_album(&current, &next));
        assert_eq!(
            AlbumDetector::should_gapless(&current, &next),
            TransitionMode::default()
        );
    }

    #[test]
    fn missing_metadata_returns_user_default() {
        let current = track(None, None, None);
        let next = track(None, None, None);
        assert!(!AlbumDetector::is_same_album(&current, &next));
        assert_eq!(
            AlbumDetector::should_gapless(&current, &next),
            TransitionMode::default()
        );
    }

    #[test]
    fn album_match_is_case_insensitive() {
        let current = track(Some("dark side of the moon"), Some("pink floyd"), Some(1));
        let next = track(Some("Dark Side of the Moon"), Some("Pink Floyd"), Some(2));
        assert!(AlbumDetector::is_same_album(&current, &next));
    }

    #[test]
    fn missing_artist_on_next_is_not_same_album() {
        let current = track(Some("Abbey Road"), Some("The Beatles"), Some(1));
        let next = track(Some("Abbey Road"), None, Some(2));
        assert!(!AlbumDetector::is_same_album(&current, &next));
    }

    // ── GaplessScheduler ─────────────────────────────────────────────────────

    fn scheduler_with_threshold(threshold_secs: f64) -> GaplessScheduler {
        GaplessScheduler::new(
            PreBuffer::new(threshold_secs, 1000),
            TransitionMode::default(),
        )
    }

    #[test]
    fn prefetch_starts_when_threshold_reached() {
        let sched = scheduler_with_threshold(10.0);
        let sample_rate = 44100_u32;
        // 9 seconds remaining → below 10-second threshold → should start
        let below = (9.0 * f64::from(sample_rate)) as u64;
        assert!(sched.should_start_prefetch(below, sample_rate));
    }

    #[test]
    fn prefetch_does_not_start_when_above_threshold() {
        let sched = scheduler_with_threshold(10.0);
        let sample_rate = 44100_u32;
        // 11 seconds remaining → above threshold → should not start
        let above = (11.0 * f64::from(sample_rate)) as u64;
        assert!(!sched.should_start_prefetch(above, sample_rate));
    }

    #[test]
    fn prefetch_does_not_repeat_when_already_active() {
        let mut sched = scheduler_with_threshold(10.0);
        let sample_rate = 44100_u32;
        let below = (5.0 * f64::from(sample_rate)) as u64;
        assert!(sched.should_start_prefetch(below, sample_rate));
        sched.mark_prefetch_started();
        // Even though we're still below threshold, should not start again
        assert!(!sched.should_start_prefetch(below, sample_rate));
    }

    #[test]
    fn cancel_prefetch_resets_active_state() {
        let mut sched = scheduler_with_threshold(10.0);
        let sample_rate = 44100_u32;
        sched.mark_prefetch_started();
        assert!(sched.is_prefetch_active());
        sched.cancel_prefetch();
        assert!(!sched.is_prefetch_active());
        // After cancel, should be able to start again
        let below = (5.0 * f64::from(sample_rate)) as u64;
        assert!(sched.should_start_prefetch(below, sample_rate));
    }

    #[test]
    fn transition_mode_selection_same_album_gapless() {
        let sched = scheduler_with_threshold(10.0);
        let current = track(Some("OK Computer"), Some("Radiohead"), Some(1));
        let next = track(Some("OK Computer"), Some("Radiohead"), Some(2));
        assert_eq!(
            sched.select_transition_mode(&current, &next),
            TransitionMode::Gapless
        );
    }

    #[test]
    fn transition_mode_selection_different_album_is_default() {
        let sched = scheduler_with_threshold(10.0);
        let current = track(Some("OK Computer"), Some("Radiohead"), Some(1));
        let next = track(Some("Kid A"), Some("Radiohead"), Some(1));
        assert_eq!(
            sched.select_transition_mode(&current, &next),
            TransitionMode::default()
        );
    }

    // ── CarryBuffer ──────────────────────────────────────────────────────────

    #[test]
    fn carry_buffer_crossfade_fade_curve_has_correct_length() {
        let buf = CarryBuffer::with_crossfade(3000, 44100);
        let curve = buf.fade_curve.as_ref().unwrap();
        // 3 seconds at 44100 Hz = 132300 per-channel positions
        assert_eq!(curve.len(), 132300);
    }

    #[test]
    fn carry_buffer_default_is_empty() {
        let buf = CarryBuffer::new();
        assert!(buf.is_empty());
        assert!(buf.fade_curve.is_none());
    }
}
