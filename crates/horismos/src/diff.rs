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

// #529 step 9: the reactive-config completeness contract. Every leaf path
// produced by `classification_leaf_paths` (below) must appear in EXACTLY one
// of `LIVE`, `RESTART_REQUIRED` (above), or `UNWIRED` — enforced by
// `every_leaf_is_classified_exactly_once`. Dynamic map keys (`taxis.libraries`)
// are canonicalized to a literal `*` segment so the set is deterministic
// regardless of what key a test/exemplar config happens to use.

/// Full dotted leaf paths wired live by #529 steps 2-8: a per-op read
/// (`Section::get()`), a swappable internal (LIVE-B), or a teardown+rebuild
/// supervisor (REBUILD) — anything that is NOT held back by
/// `RESTART_REQUIRED` and DOES have a real production consumer.
pub const LIVE: &[&str] = &[
    // exousia — step 3 (Section<ExousiaConfig>, immediate JWT rotation)
    "exousia.access_token_ttl_secs",
    "exousia.refresh_token_ttl_days",
    "exousia.jwt_secret",
    // paroche — steps 2, 4, 5 (per-op reads, renderer QUIC LiveGate, dual-listener rebind)
    "paroche.listen_addr",
    "paroche.port",
    "paroche.opds_page_size",
    "paroche.renderer_api_key",
    "paroche.kosync_registration_enabled",
    "paroche.renderer_max_connections",
    "paroche.renderer_session_init_timeout_secs",
    "paroche.renderer_quic_port",
    // taxis — step 6 (scanner rebuild supervisor)
    "taxis.libraries.*.path",
    "taxis.libraries.*.media_type",
    "taxis.libraries.*.watcher_mode",
    "taxis.libraries.*.poll_interval_seconds",
    "taxis.libraries.*.scan_interval_hours",
    "taxis.watcher_debounce_ms",
    "taxis.scan_concurrency",
    // epignosis — step 8 (resolver rebuild + swap behind MetadataAdapter)
    "epignosis.cache_ttl_secs",
    "epignosis.provider_timeout_secs",
    "epignosis.provider_response_max_bytes",
    // #578: provider credentials — rebuild supervisor re-derives
    // ProviderCredentials FROM the new section on every rebuild.
    "epignosis.acoustid_key",
    "epignosis.tmdb_key",
    "epignosis.tvdb_key",
    "epignosis.comicvine_key",
    "epignosis.google_books_key",
    // #575: merge_lookup_matches now compares against both thresholds.
    "epignosis.fingerprint_accept_threshold",
    "epignosis.fingerprint_ambiguous_threshold",
    // kritike — step 8 (LiveGate)
    "kritike.quality_check_concurrency",
    // zetesis — step 7 (per-op reads, RateLimiter::reconfigure, cf-proxy/cardigann swap)
    "zetesis.request_timeout_secs",
    "zetesis.max_response_body_bytes",
    "zetesis.cloudflare_bypass_enabled",
    "zetesis.max_concurrent_searches",
    "zetesis.per_indexer_rate_limit_requests",
    "zetesis.per_indexer_rate_limit_window_seconds",
    "zetesis.search_timeout_seconds",
    "zetesis.cardigann_definitions_dir",
    "zetesis.cf_proxy_url",
    "zetesis.cf_proxy_timeout_seconds",
    // ergasia — step 7 (SessionEngine rebuilds ExtractionLimits per call)
    "ergasia.max_extraction_depth",
    "ergasia.max_decompression_ratio",
    // syntaxis — step 7 (DownloadQueue::update_config + SlotAllocator::set_limits)
    "syntaxis.max_concurrent_downloads",
    "syntaxis.max_per_tracker",
    "syntaxis.retry_count",
    "syntaxis.retry_backoff_base_seconds",
    // prostheke — step 8 (per-op prefs, provider rebuild + set_providers)
    "prostheke.languages",
    "prostheke.include_hearing_impaired",
    "prostheke.include_forced",
    "prostheke.min_match_score",
    "prostheke.opensubtitles.api_key",
    "prostheke.opensubtitles.rate_limit_per_second",
    "prostheke.opensubtitles.max_download_bytes",
    // komide — step 6 (feed scheduler rebuild supervisor)
    "komide.podcast_poll_interval_minutes",
    "komide.news_poll_interval_minutes",
    "komide.news_retention_days",
    "komide.news_retention_articles",
    "komide.auto_download_latest_n",
    "komide.fetch_timeout_secs",
    "komide.max_feed_bytes",
    "komide.max_backoff_minutes",
    "komide.jitter_percent",
    // syndesmos — step 8 (REBUILD: client + handler respawn + adapter swap)
    "syndesmos.plex.url",
    "syndesmos.plex.token",
    "syndesmos.plex.library_sections",
    "syndesmos.lastfm.api_key",
    "syndesmos.lastfm.shared_secret",
    "syndesmos.lastfm.session_key",
    "syndesmos.tidal.access_token",
    "syndesmos.circuit_break_minutes",
    // NOTE: `circuit_break_failure_threshold` reaches the rebuild supervisor
    // (a change to it triggers the same teardown+rebuild as every other
    // syndesmos.* leaf) but `ScrobbleClientBuilder::build()` hardcodes the
    // breaker threshold to 5 internally — the config-plumbing contract this
    // test enforces is honored; the internal no-op is tracked separately
    // (see harmonia issue tracking the syndesmos hardcoded-threshold finding,
    // NOT #575's dead-config-surface list).
    "syndesmos.circuit_break_failure_threshold",
    // aitesis — step 8 (Section<AitesisConfig>, per-op reads)
    "aitesis.max_pending_per_user",
    "aitesis.max_requests_per_day",
    "aitesis.auto_approve_admins",
];

/// Full dotted leaf paths with NO production consumer — the 31 dead-config
/// fields inventoried by #529's design pass and filed as harmonia issue
/// #575. Each stays here (rather than silently vanishing from the schema)
/// until #575 dispositions it: wired to a real consumer, or removed.
pub const UNWIRED: &[&str] = &[
    "paroche.stream_buffer_kb",                 // #575
    "paroche.transcode_concurrency",            // #575
    "taxis.libraries.*.auto_import",            // #575
    "taxis.file_naming_dry_run",                // #575
    "epignosis.max_retries",                    // #575
    "kritike.scan_interval_hours",              // #575
    "aggelia.download_queue_size",              // #575
    "zetesis.max_results_per_indexer",          // #575
    "zetesis.caps_refresh_hours",               // #575
    "zetesis.cf_cookie_refresh_minutes",        // #575
    "zetesis.cf_health_check_interval_minutes", // #575
    "ergasia.max_concurrent_downloads",         // #575
    "ergasia.tracker_seed_policies", // #575 — whole TrackerSeedPolicy struct is dead; the map is empty by default and collapses to this one leaf (see `classification_leaf_paths`)
    "ergasia.progress_throttle_seconds", // #575
    "ergasia.extraction_temp_dir",   // #575
    "ergasia.max_connections_per_torrent", // #575
    "ergasia.magnet_resolve_timeout_seconds", // #575
    "ergasia.extraction_cleanup_hours", // #575
    "syntaxis.stalled_download_timeout_hours", // #575
    "prostheke.opensubtitles.username", // #575
    "prostheke.opensubtitles.password", // #575
    "komide.podcast_dir",            // #575
    "komide.max_episode_bytes",      // #575
    "syndesmos.tidal.client_id",     // #575
    "syndesmos.tidal.client_secret", // #575
    "syndesmos.tidal.refresh_token", // #575
    "syndesmos.tidal.sync_interval_minutes", // #575
    "syndesis.jitter_buffer_max_frames", // #575
    "syndesis.max_sessions",         // #575
];

// NOTE: paths a test/exemplar config walks under a dynamic-key map are
// canonicalized to this parent path with a literal `*` child segment, so
// the leaf set is deterministic regardless of the real map key used.
#[cfg(test)]
const DYNAMIC_MAP_PATHS: &[&str] = &["taxis.libraries"];

/// Single-tree leaf walker for the completeness test — a sibling of
/// `diff_value` (same Object-recursion shape) that walks ONE config tree
/// instead of diffing two, pushing every scalar/array/empty-object node as a
/// leaf. A dynamic-key map listed in `DYNAMIC_MAP_PATHS` recurses through its
/// (single, exemplar-populated) entry under a canonical `*` segment instead
/// of the real key. An empty map with no synthetic entry (e.g.
/// `ergasia.tracker_seed_policies`) has no children to recurse into, so it
/// reports its own path as one leaf — the same "dead subtree" signal a
/// removed feature would produce.
#[cfg(test)]
fn classification_leaf_paths(path: &str, value: &Value, out: &mut Vec<String>) {
    let Value::Object(map) = value else {
        out.push(path.to_string());
        return;
    };
    if map.is_empty() {
        out.push(path.to_string());
        return;
    }
    for (key, child) in map {
        let canonical_key = if DYNAMIC_MAP_PATHS.contains(&path) {
            "*"
        } else {
            key.as_str()
        };
        let child_path = if path.is_empty() {
            canonical_key.to_string()
        } else {
            format!("{path}.{canonical_key}")
        };
        classification_leaf_paths(&child_path, child, out);
    }
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

    // ── #529 step 9: classification completeness ─────────────────────────────

    // WHY: every Option<Struct> subtree is populated with `Some(..)` (rather
    // than left at its `None` default) so the leaf walker descends into its
    // fields instead of collapsing the whole subtree to one leaf — the only
    // way a mixed LIVE/UNWIRED subtree (e.g. `prostheke.opensubtitles`) gets
    // per-field classification instead of one opaque leaf. `taxis.libraries`
    // gets exactly ONE synthetic entry so `classification_leaf_paths` walks
    // it once under the canonical `*` key; `ergasia.tracker_seed_policies`
    // and `syndesmos.plex.library_sections` are deliberately left EMPTY —
    // both are uniformly classified as a whole (UNWIRED / LIVE respectively),
    // so the "empty map collapses to its own leaf" fallback is sufficient.
    fn exemplar_config() -> Config {
        use crate::subsystems::{
            LastfmConfig, LibraryConfig, OpenSubtitlesConfig, PlexConfig, TidalConfig,
        };

        let mut config = Config::default();

        config
            .taxis
            .libraries
            .insert("sample".to_string(), LibraryConfig::default());

        config.paroche.renderer_api_key = Some("sample-api-key".to_string());
        config.zetesis.cardigann_definitions_dir =
            Some(std::path::PathBuf::from("/etc/harmonia/cardigann"));
        config.zetesis.cf_proxy_url = Some("http://127.0.0.1:8191".to_string());
        config.prostheke.opensubtitles = Some(OpenSubtitlesConfig::default());
        config.syndesmos.plex = Some(PlexConfig {
            url: "http://localhost:32400".to_string(),
            token: "sample-token".to_string(),
            library_sections: std::collections::HashMap::new(),
        });
        config.syndesmos.lastfm = Some(LastfmConfig {
            api_key: "sample-key".to_string(),
            shared_secret: "sample-secret".to_string(),
            session_key: Some("sample-session".to_string()),
        });
        config.syndesmos.tidal = Some(TidalConfig {
            access_token: Some("sample-access".to_string()),
            refresh_token: Some("sample-refresh".to_string()),
            ..TidalConfig::default()
        });

        config
    }

    /// Makes an unclassified config leaf a build failure: every leaf the
    /// exemplar config produces must land in exactly one of `LIVE`,
    /// `RESTART_REQUIRED`, or `UNWIRED`, and the union of the three must
    /// exactly equal the full leaf set (disjoint + exhaustive).
    #[test]
    fn every_leaf_is_classified_exactly_once() {
        let config = exemplar_config();
        let value = serde_json::to_value(&config).unwrap(); // WHY: Config -> Value cannot fail (see diff_config)

        let mut leaves = Vec::new();
        classification_leaf_paths("", &value, &mut leaves);
        let leaf_set: BTreeSet<&str> = leaves.iter().map(String::as_str).collect();
        assert_eq!(
            leaf_set.len(),
            leaves.len(),
            "leaf walker produced duplicate paths: {leaves:?}"
        );

        let live: BTreeSet<&str> = LIVE.iter().copied().collect();
        let unwired: BTreeSet<&str> = UNWIRED.iter().copied().collect();
        assert_eq!(live.len(), LIVE.len(), "LIVE contains a duplicate entry");
        assert_eq!(
            unwired.len(),
            UNWIRED.len(),
            "UNWIRED contains a duplicate entry"
        );

        // No ghost entries: every declared LIVE/UNWIRED leaf must exist in
        // the real leaf set (catches a stale entry left behind by a removed
        // or renamed field).
        for &l in LIVE {
            assert!(
                leaf_set.contains(l),
                "LIVE lists nonexistent config leaf: {l}"
            );
        }
        for &u in UNWIRED {
            assert!(
                leaf_set.contains(u),
                "UNWIRED lists nonexistent config leaf: {u}"
            );
        }

        // Per-leaf: exactly one classification.
        let mut unclassified = Vec::new();
        let mut multi_classified = Vec::new();
        for &leaf in &leaf_set {
            let matches = [
                live.contains(leaf),
                requires_restart(leaf),
                unwired.contains(leaf),
            ]
            .iter()
            .filter(|hit| **hit)
            .count();
            match matches {
                0 => unclassified.push(leaf),
                1 => {}
                _ => multi_classified.push(leaf),
            }
        }
        assert!(
            unclassified.is_empty(),
            "config leaf(s) not in LIVE, RESTART_REQUIRED, or UNWIRED — classify the new field: {unclassified:#?}"
        );
        assert!(
            multi_classified.is_empty(),
            "config leaf(s) classified in more than one list: {multi_classified:#?}"
        );

        // Cross-check: the union must exactly equal the full leaf set.
        let restart_leaves: BTreeSet<&str> = leaf_set
            .iter()
            .copied()
            .filter(|leaf| requires_restart(leaf))
            .collect();
        let union: BTreeSet<&str> = live
            .union(&restart_leaves)
            .copied()
            .collect::<BTreeSet<&str>>()
            .union(&unwired)
            .copied()
            .collect();
        assert_eq!(
            union, leaf_set,
            "LIVE ∪ RESTART_REQUIRED ∪ UNWIRED must exactly equal the full config leaf set"
        );
    }
}
