use std::collections::BTreeSet;

use serde_json::Value;
use snafu::{OptionExt, ResultExt};

use crate::Config;
use crate::error::{HorismosError, MergePathSnafu, MergeRoundTripSnafu};

// WHY: pure data — change record for config diff reporting.
pub struct ConfigChange {
    pub path: String,
    pub requires_restart: bool,
}

// NOTE: path PREFIXES — a changed leaf is restart-class when its dotted path
// starts with any entry. `database.` (trailing dot) covers the whole section.
// `exousia` is deliberately absent: auth config goes live in a later step.
const RESTART_REQUIRED: &[&str] = &[
    "database.",
    "aggelia.buffer_size",
    "ergasia.download_dir",
    "ergasia.session_state_path",
    "ergasia.listen_port_range",
    "ergasia.peer_connect_timeout_seconds",
    // #529 step 7: frozen into librqbit's `SeedingPolicy` at session build with
    // no reconfigure API — holding these back keeps `current()` honest (a
    // reload would otherwise "apply" a value with zero live effect).
    "ergasia.seed_ratio_threshold",
    "ergasia.seed_time_threshold_hours",
];

fn requires_restart(path: &str) -> bool {
    RESTART_REQUIRED
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

/// Compare two configs and return the changed leaves as dotted paths
/// (e.g. `paroche.port`, `taxis.libraries.music.path`), sorted.
pub fn diff_config(old: &Config, new: &Config) -> Vec<ConfigChange> {
    let old_val = serde_json::to_value(old).unwrap_or_default(); // WHY: serde_json::to_value on a statically-typed Config struct cannot fail
    let new_val = serde_json::to_value(new).unwrap_or_default(); // WHY: serde_json::to_value on a statically-typed Config struct cannot fail

    let mut paths = Vec::new();
    diff_value("", &old_val, &new_val, &mut paths);
    paths
        .into_iter()
        .map(|path| ConfigChange {
            requires_restart: requires_restart(&path),
            path,
        })
        .collect()
}

// NOTE: arrays are atomic leaves (`ergasia.listen_port_range` reports one
// path, not per-index paths). A subtree appearing or disappearing (e.g. an
// Option section flipping None <-> Some) reports the subtree path itself.
// Map keys containing '.' would produce ambiguous dotted paths, but no
// restart-class prefix covers a dynamic-key map (taxis), so the held-back
// copy never walks one.
fn diff_value(path: &str, old: &Value, new: &Value, out: &mut Vec<String>) {
    match (old, new) {
        (Value::Object(old_map), Value::Object(new_map)) => {
            let keys: BTreeSet<&String> = old_map.keys().chain(new_map.keys()).collect();
            for key in keys {
                let child = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                diff_value(
                    &child,
                    old_map.get(key).unwrap_or(&Value::Null),
                    new_map.get(key).unwrap_or(&Value::Null),
                    out,
                );
            }
        }
        _ => {
            if old != new {
                out.push(path.to_string());
            }
        }
    }
}

/// Produce the new EFFECTIVE config from the old effective config and the new
/// on-disk config: every restart-class changed leaf keeps its OLD (effective)
/// value; every other leaf takes the new value. Generic over the config shape
/// — leaves are copied through the serde_json tree, no per-field code.
pub(crate) fn held_back_merge(old: &Config, new: &Config) -> Result<Config, HorismosError> {
    let old_val = serde_json::to_value(old).context(MergeRoundTripSnafu)?;
    let mut new_val = serde_json::to_value(new).context(MergeRoundTripSnafu)?;

    let mut changed = Vec::new();
    diff_value("", &old_val, &new_val, &mut changed);
    for path in changed.iter().filter(|p| requires_restart(p)) {
        copy_leaf(&old_val, &mut new_val, path)?;
    }

    serde_json::from_value(new_val).context(MergeRoundTripSnafu)
}

// INVARIANT: both trees serialize from the same statically-typed Config, so a
// path the diff found exists in both; a miss surfaces as MergePath rather
// than silently dropping the held-back guarantee.
fn copy_leaf(from: &Value, to: &mut Value, path: &str) -> Result<(), HorismosError> {
    let mut from_cur = from;
    let mut to_cur = to;
    let mut segments = path.split('.').peekable();
    while let Some(segment) = segments.next() {
        let from_next = from_cur.get(segment).context(MergePathSnafu { path })?;
        let to_next = to_cur.get_mut(segment).context(MergePathSnafu { path })?;
        if segments.peek().is_none() {
            *to_next = from_next.clone();
            return Ok(());
        }
        from_cur = from_next;
        to_cur = to_next;
    }
    MergePathSnafu { path }.fail()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use crate::subsystems::LibraryConfig;

    fn base_config() -> Config {
        let mut c = Config::default();
        c.exousia.jwt_secret = "a-very-long-secret-key-that-is-at-least-32-bytes-long".into();
        c
    }

    fn paths(changes: &[ConfigChange]) -> Vec<&str> {
        changes.iter().map(|c| c.path.as_str()).collect()
    }

    // ── diff_config: leaf paths ───────────────────────────────────────────────

    #[test]
    fn identical_configs_return_no_changes() {
        let c = base_config();
        assert!(diff_config(&c, &c).is_empty());
    }

    #[test]
    fn changed_paroche_port_returns_leaf_path_non_restart() {
        let old = base_config();
        let mut new = base_config();
        new.paroche.port = 9090;

        let changes = diff_config(&old, &new);
        assert_eq!(paths(&changes), vec!["paroche.port"]);
        assert!(!changes[0].requires_restart);
    }

    #[test]
    fn changed_database_path_returns_restart_required() {
        let old = base_config();
        let mut new = base_config();
        new.database.db_path = std::path::PathBuf::from("/new/path/harmonia.db");

        let changes = diff_config(&old, &new);
        assert_eq!(paths(&changes), vec!["database.db_path"]);
        assert!(changes[0].requires_restart);
    }

    #[test]
    fn changed_exousia_is_not_restart_class() {
        let old = base_config();
        let mut new = base_config();
        new.exousia.jwt_secret = "another-very-long-secret-key-that-is-32-bytes-plus".into();

        let changes = diff_config(&old, &new);
        assert_eq!(paths(&changes), vec!["exousia.jwt_secret"]);
        assert!(!changes[0].requires_restart);
    }

    #[test]
    fn nested_library_change_returns_dotted_leaf_path() {
        let mut old = base_config();
        old.taxis.libraries.insert(
            "music".to_string(),
            LibraryConfig {
                path: std::path::PathBuf::from("/data/music"),
                ..LibraryConfig::default()
            },
        );
        let mut new = base_config();
        new.taxis.libraries.insert(
            "music".to_string(),
            LibraryConfig {
                path: std::path::PathBuf::from("/mnt/music"),
                ..LibraryConfig::default()
            },
        );

        let changes = diff_config(&old, &new);
        assert_eq!(paths(&changes), vec!["taxis.libraries.music.path"]);
        assert!(!changes[0].requires_restart);
    }

    #[test]
    fn added_library_reports_subtree_path() {
        let old = base_config();
        let mut new = base_config();
        new.taxis.libraries.insert(
            "music".to_string(),
            LibraryConfig {
                path: std::path::PathBuf::from("/data/music"),
                ..LibraryConfig::default()
            },
        );

        let changes = diff_config(&old, &new);
        assert_eq!(paths(&changes), vec!["taxis.libraries.music"]);
    }

    #[test]
    fn changed_listen_port_range_is_one_restart_leaf() {
        let old = base_config();
        let mut new = base_config();
        new.ergasia.listen_port_range = [7000, 7009];

        let changes = diff_config(&old, &new);
        assert_eq!(paths(&changes), vec!["ergasia.listen_port_range"]);
        assert!(changes[0].requires_restart);
    }

    #[test]
    fn changed_seed_ratio_threshold_is_restart_class() {
        let old = base_config();
        let mut new = base_config();
        new.ergasia.seed_ratio_threshold = 2.5;

        let changes = diff_config(&old, &new);
        assert_eq!(paths(&changes), vec!["ergasia.seed_ratio_threshold"]);
        assert!(changes[0].requires_restart);
    }

    #[test]
    fn changed_seed_time_threshold_hours_is_restart_class() {
        let old = base_config();
        let mut new = base_config();
        new.ergasia.seed_time_threshold_hours = 200;

        let changes = diff_config(&old, &new);
        assert_eq!(paths(&changes), vec!["ergasia.seed_time_threshold_hours"]);
        assert!(changes[0].requires_restart);
    }

    #[test]
    fn multiple_changed_leaves_return_multiple_entries() {
        let old = base_config();
        let mut new = base_config();
        new.paroche.port = 9090;
        new.kritike.scan_interval_hours = 12;

        let changes = diff_config(&old, &new);
        assert_eq!(
            paths(&changes),
            vec!["kritike.scan_interval_hours", "paroche.port"]
        );
        assert!(changes.iter().all(|c| !c.requires_restart));
    }

    // ── held_back_merge ───────────────────────────────────────────────────────

    #[test]
    fn merge_holds_back_restart_leaf_and_applies_live_leaf() {
        let old = base_config();
        let mut new = base_config();
        new.database.db_path = std::path::PathBuf::from("/new/harmonia.db");
        new.paroche.port = 9090;

        let effective = held_back_merge(&old, &new).unwrap();
        assert_eq!(effective.database.db_path, old.database.db_path);
        assert_eq!(effective.paroche.port, 9090);
    }

    #[test]
    fn merge_without_restart_changes_equals_new() {
        let old = base_config();
        let mut new = base_config();
        new.paroche.port = 9090;
        new.kritike.scan_interval_hours = 12;

        let effective = held_back_merge(&old, &new).unwrap();
        assert!(diff_config(&effective, &new).is_empty());
    }

    #[test]
    fn merge_holds_back_every_leaf_under_database_prefix() {
        let old = base_config();
        let mut new = base_config();
        new.database.db_path = std::path::PathBuf::from("/new/harmonia.db");
        new.database.write_pool_max = 8;

        let effective = held_back_merge(&old, &new).unwrap();
        assert_eq!(effective.database.db_path, old.database.db_path);
        assert_eq!(
            effective.database.write_pool_max,
            old.database.write_pool_max
        );
    }

    #[test]
    fn merge_holds_back_seed_threshold_changes() {
        let old = base_config();
        let mut new = base_config();
        new.ergasia.seed_ratio_threshold = 3.0;
        new.ergasia.seed_time_threshold_hours = 999;
        new.paroche.port = 9090;

        let effective = held_back_merge(&old, &new).unwrap();
        assert_eq!(
            effective.ergasia.seed_ratio_threshold,
            old.ergasia.seed_ratio_threshold
        );
        assert_eq!(
            effective.ergasia.seed_time_threshold_hours,
            old.ergasia.seed_time_threshold_hours
        );
        assert_eq!(effective.paroche.port, 9090);
    }
}
