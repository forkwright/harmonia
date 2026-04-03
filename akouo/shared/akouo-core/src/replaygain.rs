// ReplayGain audio normalization

pub struct ReplayGain {
    track_gain: f32,
    #[expect(dead_code, reason = "peak value stored for future clipping prevention; not yet read in apply_track_gain")]
    track_peak: f32,
    album_gain: f32,
    #[expect(dead_code, reason = "peak value stored for future clipping prevention; not yet read in apply_album_gain")]
    album_peak: f32,
}

impl ReplayGain {
    pub fn new(track_gain: f32, track_peak: f32, album_gain: f32, album_peak: f32) -> Self {
        Self {
            track_gain,
            track_peak,
            album_gain,
            album_peak,
        }
    }

    pub fn apply_track_gain(&self, samples: &mut [i16]) {
        let gain_db = self.track_gain;
        let gain_linear = 10_f32.powf(gain_db / 20.0);

        for sample in samples.iter_mut() {
            let adjusted = (*f32::try_from(sample).unwrap_or_default()) * gain_linear;
            *sample = adjusted.clamp(i16::f32::try_from(MIN).unwrap_or_default(), i16::f32::try_from(MAX).unwrap_or_default()) as i16;
        }
    }

    pub fn apply_album_gain(&self, samples: &mut [i16]) {
        let gain_db = self.album_gain;
        let gain_linear = 10_f32.powf(gain_db / 20.0);

        for sample in samples.iter_mut() {
            let adjusted = (*f32::try_from(sample).unwrap_or_default()) * gain_linear;
            *sample = adjusted.clamp(i16::f32::try_from(MIN).unwrap_or_default(), i16::f32::try_from(MAX).unwrap_or_default()) as i16;
        }
    }
}
