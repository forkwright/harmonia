use snafu::ResultExt;
use sqlx::SqlitePool;

use crate::error::{DatabaseSnafu, KritikeError, ProfileNotFoundSnafu};
use harmonia_db::repo::quality;

/// A resolved quality profile FROM the database.
#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    pub id: i64,
    pub name: String,
    pub media_type: String,
    pub min_quality_score: i32,
    pub upgrade_until_score: i32,
    pub min_custom_format_score: i32,
    pub upgrade_until_format_score: i32,
    pub upgrades_allowed: bool,
}

pub async fn load_profile(
    pool: &SqlitePool,
    profile_id: i64,
) -> Result<ResolvedProfile, KritikeError> {
    let row = quality::get_profile(pool, profile_id)
        .await
        .context(DatabaseSnafu)?
        .ok_or_else(|| ProfileNotFoundSnafu { id: profile_id }.build())?;

    Ok(ResolvedProfile {
        id: row.id,
        name: row.name,
        media_type: row.media_type,
        min_quality_score: i32::try_from(row.min_quality_score).unwrap_or_default(),
        upgrade_until_score: i32::try_from(row.upgrade_until_score).unwrap_or_default(),
        min_custom_format_score: i32::try_from(row.min_custom_format_score).unwrap_or_default(),
        upgrade_until_format_score: i32::try_from(row.upgrade_until_format_score).unwrap_or_default(),
        upgrades_allowed: row.upgrades_allowed != 0,
    })
}
