use crate::config::{ReplayGainConfig, ReplayGainMode};
use crate::dsp::{DspStage, StageResult};
use crate::signal_path::{SignalStageInfo, StageParams};

pub struct ReplayGainStage {
    config: ReplayGainConfig,
    /// Gain actually applied to the current track, in dB. Stored for signal path display.
    applied_gain_db: f64,
}

impl ReplayGainStage {
    pub fn new(config: ReplayGainConfig) -> Self {
        let applied_gain_db = Self::compute_gain_db(&config);
        Self {
            config,
            applied_gain_db,
        }
    }

    /// Select the base gain (dB) from tag metadata according to the configured mode.
    fn selected_gain_db(config: &ReplayGainConfig) -> f64 {
        match config.mode {
            ReplayGainMode::Track => config.track_gain_db.unwrap_or(config.fallback_gain_db),
            ReplayGainMode::Album => config
                .album_gain_db
                .or_else(|| {
                    config
                        .fallback_to_track
                        .then_some(config.track_gain_db)
                        .flatten()
                })
                .unwrap_or(config.fallback_gain_db),
            ReplayGainMode::R128 => {
                // Prefer R128 track gain, fall back to legacy track gain, then fallback.
                config
                    .r128_track_gain
                    .or(config.track_gain_db)
                    .unwrap_or(config.fallback_gain_db)
            }
        }
    }

    /// Select the peak value corresponding to the active gain mode.
    fn selected_peak(config: &ReplayGainConfig) -> Option<f64> {
        match config.mode {
            ReplayGainMode::Album => config.album_peak.or(config.track_peak),
            _ => config.track_peak,
        }
    }

    /// Compute the total gain (dB) including preamp and clipping prevention.
    fn compute_gain_db(config: &ReplayGainConfig) -> f64 {
        let base_db = Self::selected_gain_db(config) + config.preamp_db;
        let mut gain_linear = 10f64.powf(base_db / 20.0);

        if config.prevent_clipping
            && let Some(peak) = Self::selected_peak(config)
        {
            let peak = peak.abs();
            if peak > 0.0 && peak * gain_linear > 1.0 {
                gain_linear = 1.0 / peak;
            }
        }

        20.0 * gain_linear.log10()
    }
}

impl DspStage for ReplayGainStage {
    fn name(&self) -> &str {
        "replaygain"
    }

    fn process(&mut self, samples: &mut [f64], _channels: u16, _sample_rate: u32) -> StageResult {
        if self.config.enabled {
            // Recompute in case config was hot-swapped (pipeline rebuild resets state anyway,
            // but computing here keeps applied_gain_db accurate for signal_stage_meta).
            self.applied_gain_db = Self::compute_gain_db(&self.config);
            let gain_linear = 10f64.powf(self.applied_gain_db / 20.0);

            for s in samples.iter_mut() {
                *s *= gain_linear;
            }
        }

        StageResult {
            meta: self.signal_stage_meta(),
        }
    }

    fn signal_stage_meta(&self) -> SignalStageInfo {
        SignalStageInfo {
            name: self.name().to_owned(),
            enabled: self.config.enabled,
            params: StageParams::ReplayGain {
                mode: format!("{:?}", self.config.mode),
                gain_db: self.applied_gain_db,
            },
            tier_impact: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ReplayGainMode;

    fn cfg_with_track(track_gain_db: f64, track_peak: f64) -> ReplayGainConfig {
        ReplayGainConfig {
            enabled: true,
            mode: ReplayGainMode::Track,
            preamp_db: 0.0,
            fallback_to_track: true,
            fallback_gain_db: 0.0,
            prevent_clipping: false,
            track_gain_db: Some(track_gain_db),
            track_peak: Some(track_peak),
            album_gain_db: None,
            album_peak: None,
            r128_track_gain: None,
            r128_album_gain: None,
        }
    }

    fn apply(config: ReplayGainConfig, input: f64) -> f64 {
        let mut stage = ReplayGainStage::new(config);
        let mut buf = vec![input];
        stage.process(&mut buf, 1, 44100);
        buf[0]
    }

    #[test]
    fn track_mode_applies_track_gain() {
        let gain_db = -6.0_f64;
        let gain_linear = 10f64.powf(gain_db / 20.0);
        let result = apply(cfg_with_track(gain_db, 0.5), 1.0);
        assert!((result - gain_linear).abs() < 1e-10);
    }

    #[test]
    fn album_mode_applies_album_gain() {
        let config = ReplayGainConfig {
            enabled: true,
            mode: ReplayGainMode::Album,
            preamp_db: 0.0,
            fallback_to_track: true,
            fallback_gain_db: 0.0,
            prevent_clipping: false,
            track_gain_db: Some(-3.0),
            track_peak: Some(0.5),
            album_gain_db: Some(-9.0),
            album_peak: Some(0.8),
            r128_track_gain: None,
            r128_album_gain: None,
        };
        let expected = 10f64.powf(-9.0_f64 / 20.0);
        let result = apply(config, 1.0);
        assert!((result - expected).abs() < 1e-10);
    }

    #[test]
    fn album_mode_falls_back_to_track_when_no_album_gain() {
        let config = ReplayGainConfig {
            enabled: true,
            mode: ReplayGainMode::Album,
            preamp_db: 0.0,
            fallback_to_track: true,
            fallback_gain_db: 0.0,
            prevent_clipping: false,
            track_gain_db: Some(-5.0),
            track_peak: Some(0.5),
            album_gain_db: None,
            album_peak: None,
            r128_track_gain: None,
            r128_album_gain: None,
        };
        let expected = 10f64.powf(-5.0_f64 / 20.0);
        let result = apply(config, 1.0);
        assert!(
            (result - expected).abs() < 1e-10,
            "should fall back to track gain"
        );
    }

    #[test]
    fn no_tags_fallback_zero_db_leaves_signal_unchanged() {
        let config = ReplayGainConfig {
            enabled: true,
            mode: ReplayGainMode::Track,
            preamp_db: 0.0,
            fallback_to_track: true,
            fallback_gain_db: 0.0,
            prevent_clipping: false,
            track_gain_db: None,
            track_peak: None,
            album_gain_db: None,
            album_peak: None,
            r128_track_gain: None,
            r128_album_gain: None,
        };
        let result = apply(config, 0.7);
        assert!(
            (result - 0.7).abs() < 1e-10,
            "no tags + 0 dB fallback should be transparent"
        );
    }

    #[test]
    fn clipping_prevention_limits_gain() {
        // gain = +20 dB (linear 10×), peak = 0.5 → 10 × 0.5 = 5.0 > 1.0 → should limit
        let config = ReplayGainConfig {
            enabled: true,
            mode: ReplayGainMode::Track,
            preamp_db: 0.0,
            fallback_to_track: true,
            fallback_gain_db: 0.0,
            prevent_clipping: true,
            track_gain_db: Some(20.0),
            track_peak: Some(0.5),
            album_gain_db: None,
            album_peak: None,
            r128_track_gain: None,
            r128_album_gain: None,
        };
        let result = apply(config, 0.5);
        assert!(
            result <= 1.0 + 1e-9,
            "clipping prevention should keep output ≤ 1.0, got {result}"
        );
        // The actual gain should be 1/peak = 2.0×
        assert!(
            (result - 1.0).abs() < 1e-9,
            "peak sample should be brought to exactly 1.0"
        );
    }

    #[test]
    fn clipping_prevention_no_effect_when_gain_safe() {
        // gain = -6 dB, peak = 0.9 → gain_linear ≈ 0.501, 0.501 × 0.9 ≈ 0.451 < 1.0
        let config = ReplayGainConfig {
            enabled: true,
            mode: ReplayGainMode::Track,
            preamp_db: 0.0,
            fallback_to_track: true,
            fallback_gain_db: 0.0,
            prevent_clipping: true,
            track_gain_db: Some(-6.0),
            track_peak: Some(0.9),
            album_gain_db: None,
            album_peak: None,
            r128_track_gain: None,
            r128_album_gain: None,
        };
        let expected = 10f64.powf(-6.0_f64 / 20.0);
        let result = apply(config, 1.0);
        assert!(
            (result - expected).abs() < 1e-9,
            "no clipping risk → gain should be applied as-is"
        );
    }

    #[test]
    fn r128_gain_applied() {
        let r128_gain = -8.0_f64;
        let config = ReplayGainConfig {
            enabled: true,
            mode: ReplayGainMode::R128,
            preamp_db: 0.0,
            fallback_to_track: true,
            fallback_gain_db: 0.0,
            prevent_clipping: false,
            track_gain_db: Some(-3.0),
            track_peak: Some(0.5),
            album_gain_db: None,
            album_peak: None,
            r128_track_gain: Some(r128_gain),
            r128_album_gain: None,
        };
        let expected = 10f64.powf(r128_gain / 20.0);
        let result = apply(config, 1.0);
        assert!(
            (result - expected).abs() < 1e-10,
            "R128 mode should use r128_track_gain"
        );
    }

    #[test]
    fn disabled_passes_through() {
        let mut config = cfg_with_track(-12.0, 0.5);
        config.enabled = false;
        let result = apply(config, 0.8);
        assert!((result - 0.8).abs() < 1e-10);
    }

    #[test]
    fn preamp_added_to_gain() {
        let config = ReplayGainConfig {
            enabled: true,
            mode: ReplayGainMode::Track,
            preamp_db: 3.0,
            fallback_to_track: true,
            fallback_gain_db: 0.0,
            prevent_clipping: false,
            track_gain_db: Some(-6.0),
            track_peak: Some(0.1),
            album_gain_db: None,
            album_peak: None,
            r128_track_gain: None,
            r128_album_gain: None,
        };
        let expected = 10f64.powf((-6.0_f64 + 3.0) / 20.0);
        let result = apply(config, 1.0);
        assert!(
            (result - expected).abs() < 1e-9,
            "preamp should be added to track gain"
        );
    }
}
