use serde::{Deserialize, Serialize};

use crate::subsystems::{
    AggeliaConfig, DatabaseConfig, EpignosisConfig, ErgasiaConfig, ExousiaConfig, KomideConfig,
    KritikeConfig, ParocheConfig, ProsthekeConfig, SyntaxisConfig, TaxisConfig, ZetesisConfig,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub exousia: ExousiaConfig,
    #[serde(default)]
    pub paroche: ParocheConfig,
    #[serde(default)]
    pub taxis: TaxisConfig,
    #[serde(default)]
    pub epignosis: EpignosisConfig,
    #[serde(default)]
    pub kritike: KritikeConfig,
    #[serde(default)]
    pub aggelia: AggeliaConfig,
    #[serde(default)]
    pub zetesis: ZetesisConfig,
    #[serde(default)]
    pub ergasia: ErgasiaConfig,
    #[serde(default)]
    pub syntaxis: SyntaxisConfig,
    #[serde(default)]
    pub prostheke: ProsthekeConfig,
    #[serde(default)]
    pub komide: KomideConfig,
}
