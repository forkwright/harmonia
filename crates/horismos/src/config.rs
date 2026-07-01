use serde::{Deserialize, Serialize};

use crate::subsystems::{
    AggeliaConfig, AitesisConfig, DatabaseConfig, EpignosisConfig, ErgasiaConfig, ExousiaConfig,
    KomideConfig, KritikeConfig, ParocheConfig, ProsthekeConfig, SearchSubsystemConfig,
    SyndesisConfig, SyndesmosConfig, SyntaxisConfig, TaxisConfig,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    pub zetesis: SearchSubsystemConfig,
    #[serde(default)]
    pub ergasia: ErgasiaConfig,
    #[serde(default)]
    pub syntaxis: SyntaxisConfig,
    #[serde(default)]
    pub prostheke: ProsthekeConfig,
    #[serde(default)]
    pub komide: KomideConfig,
    #[serde(default)]
    pub syndesmos: SyndesmosConfig,
    #[serde(default)]
    pub syndesis: SyndesisConfig,
    #[serde(default)]
    pub aitesis: AitesisConfig,
}
