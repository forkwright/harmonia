use serde::{Deserialize, Serialize};

use super::config::{EqBand, FilterType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqPreset {
    pub name: String,
    pub bands: Vec<EqBand>,
    pub preamp_db: f64,
}

fn peaking(frequency: f64, gain_db: f64) -> EqBand {
    EqBand {
        frequency,
        gain_db,
        q: 1.414,
        filter_type: FilterType::Peaking,
        enabled: true,
    }
}

pub fn built_in_presets() -> Vec<EqPreset> {
    vec![
        EqPreset {
            name: "Flat".into(),
            preamp_db: 0.0,
            bands: vec![
                peaking(31.0, 0.0),
                peaking(63.0, 0.0),
                peaking(125.0, 0.0),
                peaking(250.0, 0.0),
                peaking(500.0, 0.0),
                peaking(1000.0, 0.0),
                peaking(2000.0, 0.0),
                peaking(4000.0, 0.0),
                peaking(8000.0, 0.0),
                peaking(16000.0, 0.0),
            ],
        },
        EqPreset {
            name: "Bass Boost".into(),
            preamp_db: -4.0,
            bands: vec![
                peaking(31.0, 4.0),
                peaking(63.0, 4.0),
                peaking(125.0, 3.0),
                peaking(250.0, 2.0),
                peaking(500.0, 0.0),
                peaking(1000.0, 0.0),
                peaking(2000.0, 0.0),
                peaking(4000.0, 0.0),
                peaking(8000.0, 0.0),
                peaking(16000.0, 0.0),
            ],
        },
        EqPreset {
            name: "Treble Boost".into(),
            preamp_db: -4.0,
            bands: vec![
                peaking(31.0, 0.0),
                peaking(63.0, 0.0),
                peaking(125.0, 0.0),
                peaking(250.0, 0.0),
                peaking(500.0, 0.0),
                peaking(1000.0, 0.0),
                peaking(2000.0, 1.0),
                peaking(4000.0, 2.0),
                peaking(8000.0, 3.0),
                peaking(16000.0, 4.0),
            ],
        },
        EqPreset {
            name: "Loudness".into(),
            preamp_db: -4.0,
            bands: vec![
                peaking(31.0, 4.0),
                peaking(63.0, 3.0),
                peaking(125.0, 2.0),
                peaking(250.0, 0.0),
                peaking(500.0, -1.0),
                peaking(1000.0, -2.0),
                peaking(2000.0, -1.0),
                peaking(4000.0, 0.0),
                peaking(8000.0, 2.0),
                peaking(16000.0, 3.0),
            ],
        },
        EqPreset {
            name: "Vocal".into(),
            preamp_db: -2.0,
            bands: vec![
                peaking(31.0, 0.0),
                peaking(63.0, 0.0),
                peaking(125.0, 0.0),
                peaking(250.0, 0.0),
                peaking(500.0, 1.0),
                peaking(1000.0, 2.0),
                peaking(2000.0, 2.0),
                peaking(4000.0, 1.0),
                peaking(8000.0, 0.0),
                peaking(16000.0, 0.0),
            ],
        },
        EqPreset {
            name: "Late Night".into(),
            preamp_db: -3.0,
            bands: vec![
                peaking(31.0, -2.0),
                peaking(63.0, 1.0),
                peaking(125.0, 2.0),
                peaking(250.0, 3.0),
                peaking(500.0, 1.0),
                peaking(1000.0, 0.0),
                peaking(2000.0, -1.0),
                peaking(4000.0, -1.0),
                peaking(8000.0, 0.0),
                peaking(16000.0, 0.0),
            ],
        },
    ]
}
