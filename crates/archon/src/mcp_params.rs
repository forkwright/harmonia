//! Typed MCP tool parameter DTOs (#652, PR 1 of the rmcp migration). // kanon:ignore STORAGE/no-migration-checksum -- false positive: this module holds wire DTOs for the db_migrate TOOL; no migration code or checksum-relevant logic lives here
//!
//! One struct per tool on the stdio surface — the offline tools
//! (`harmonia_db_migrate`, `harmonia_migrate_library`, the blocking
//! `harmonia_play_file`/`harmonia_render`, and the PR-4 lifecycle trios
//! `harmonia_play_file_start`/`_status`/`_stop` and
//! `harmonia_render_start`/`_status`/`_stop`) and the 4 acquisition tools
//! forwarded to the serve-hosted bridge (`harmonia_search_releases`,
//! `harmonia_enqueue_download`, `harmonia_list_downloads`,
//! `harmonia_cancel_download`). The lifecycle `*_start` tools reuse
//! `PlayFileParams`/`RenderParams` verbatim — the spawned work takes exactly
//! the blocking call's arguments; only the status/stop op-id DTOs are new
//! shapes. Each struct pins the arguments the live surface accepts; the
//! tests in this file are the parity evidence.
//!
//! The rmcp stdio server (PR 2) takes these structs as `#[tool]` parameter
//! types, so their schemars-generated `inputSchema` IS the advertised
//! contract. Where the pre-rmcp hand-written schema and its live parser
//! disagreed, the struct followed the live parser (a client conforming to
//! the advertised schema is always accepted); the deltas PR 2 normalized:
//!
//! - Unknown object keys are ignored at every layer today; the advertised
//!   `additionalProperties: false` was never enforced by any parser on the
//!   path, so these structs do not deny unknown fields either.
//! - `migrate_library`'s boolean flags treat explicit `null` as `false`
//!   (the hand-rolled `optional_bool` does), where the schema says boolean.
//! - `search_releases.media_type` and `list_downloads.status` accept any
//!   string at the DTO layer; the advertised enums are validated downstream
//!   (the bridge handler rejects an unknown `status`; `media_type` passes
//!   through to the search service).
//! - `enqueue_download.priority` and `list_downloads.limit` accept
//!   out-of-range integers that the bridge then clamps; `want_id` deserializes
//!   as optional because the "required" rejection is a bridge-level tool error
//!   today. (`search_releases.limit` is forwarded unclamped.)
//! - Non-object `arguments`: the pre-rmcp `Value::get` extraction treated
//!   explicit `null`, arrays, and scalars as all-keys-absent. Since PR 2,
//!   explicit `null`/absent runs a tool with defaults, but any other
//!   non-object `arguments` value fails typed request dispatch and is
//!   answered as an unknown method (`-32601`) before any tool runs (witness
//!   finding on #700).

use std::net::SocketAddr;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// WHY: the pre-rmcp extractor (`optional_bool` in the old hand-rolled
/// `mcp.rs`) treated a missing flag and an explicit `null` identically —
/// both mean `false`. Plain `#[serde(default)]` on a `bool` field rejects
/// explicit null, so the two `migrate_library` flags keep the tolerant
/// behavior by deserializing through `Option` first.
fn null_tolerant_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<bool>::deserialize(deserializer)?.unwrap_or(false))
}

fn default_search_limit() -> u32 {
    100
}

fn default_enqueue_priority() -> u8 {
    4
}

fn default_list_limit() -> u32 {
    50
}

/// Media types `harmonia_migrate_library` accepts, mirrored from the live
/// extractor's closed match (`mcp.rs` `required_media_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum MigrateMediaType {
    /// Music library.
    Music,
    /// Ebook library.
    Books,
    /// Audiobook library.
    Audiobooks,
    /// Podcast library.
    Podcasts,
}

/// Parameters for `harmonia_db_migrate` — applies embedded SQLite migrations.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct DbMigrateParams {
    /// Path to harmonia.toml. Defaults to harmonia.toml.
    pub config: Option<PathBuf>,
}

/// Parameters for `harmonia_migrate_library` — runs the canonical storage
/// migrator for one media type.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct MigrateLibraryParams {
    /// Source directory containing legacy media.
    pub source: PathBuf,
    /// Target directory for canonical output.
    pub target: PathBuf,
    /// One of music, books, audiobooks, podcasts.
    pub media_type: MigrateMediaType,
    /// Report what would change without writing.
    #[serde(default, deserialize_with = "null_tolerant_bool")]
    pub dry_run: bool,
    /// Copy files instead of moving them.
    #[serde(default, deserialize_with = "null_tolerant_bool")]
    pub copy: bool,
}

/// Parameters for `harmonia_play_file` and `harmonia_play_file_start` —
/// plays one local file through the Akouo audio engine. The blocking tool
/// awaits the whole track; the start tool spawns it as a registry op.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct PlayFileParams {
    /// Path to the audio file.
    pub file: PathBuf,
    /// Optional audio output device name.
    pub device: Option<String>,
}

/// Parameters for `harmonia_render` and `harmonia_render_start` — runs the
/// headless renderer loop. The blocking tool awaits the renderer; the start
/// tool spawns it as a registry op.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct RenderParams {
    /// Optional host:port server address.
    pub server: Option<SocketAddr>,
    /// Directory for TLS certificates and pairing credentials. Defaults to $XDG_CONFIG_HOME/harmonia/renderer.
    pub cert_dir: Option<PathBuf>,
    /// Optional renderer display name.
    pub name: Option<String>,
    /// Optional renderer TOML config path.
    pub config: Option<PathBuf>,
}

/// Parameters for `harmonia_play_file_status` — reports the playback op's
/// state (running / exited with its summary / idle).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct PlayFileStatusParams {
    /// Op id returned by harmonia_play_file_start. When omitted, describes the running op, else the most recent exit. When given, must name one of those two — anything else is a tool error.
    pub op_id: Option<String>,
}

/// Parameters for `harmonia_play_file_stop` — cancels the running playback
/// op's token and awaits its teardown.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct PlayFileStopParams {
    /// Op id returned by harmonia_play_file_start. When omitted, stops the running op. When given, it must BE the running op — a mismatch refuses without stopping anything.
    pub op_id: Option<String>,
}

/// Parameters for `harmonia_render_status` — reports the renderer op's
/// state (running / exited with its summary / idle).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct RenderStatusParams {
    /// Op id returned by harmonia_render_start. When omitted, describes the running op, else the most recent exit. When given, must name one of those two — anything else is a tool error.
    pub op_id: Option<String>,
}

/// Parameters for `harmonia_render_stop` — cancels the running renderer
/// op's token and awaits its teardown.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct RenderStopParams {
    /// Op id returned by harmonia_render_start. When omitted, stops the running op. When given, it must BE the running op — a mismatch refuses without stopping anything.
    pub op_id: Option<String>,
}

/// Parameters for `harmonia_search_releases` — searches configured
/// acquisition indexers. Mirrors the bridge's own acceptance
/// (`paroche::routes::search::SearchRequest`) so a call validated here still
/// parses when re-parsed bridge-side.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct SearchReleasesParams {
    /// Free-text query.
    pub query_text: Option<String>,
    /// One of any, tv, movie, music, book — validated downstream, not here.
    pub media_type: Option<String>,
    /// Artist filter (music).
    pub artist: Option<String>,
    /// Album filter (music).
    pub album: Option<String>,
    /// Author filter (books).
    pub author: Option<String>,
    /// IMDb id filter (video).
    pub imdb_id: Option<String>,
    /// TVDB id filter (tv).
    pub tvdb_id: Option<u32>,
    /// TMDB id filter (movie).
    pub tmdb_id: Option<u32>,
    /// Season number filter (tv).
    pub season: Option<u32>,
    /// Episode number filter (tv).
    pub episode: Option<u32>,
    /// Newznab/Torznab category id filter.
    #[serde(default)]
    pub category_ids: Vec<u32>,
    /// Maximum results to return.
    #[serde(default = "default_search_limit")]
    pub limit: u32,
    /// Results to skip.
    #[serde(default)]
    pub offset: u32,
}

/// Parameters for `harmonia_enqueue_download` — enqueues exactly one of a
/// cached search result (`release_id`) or a magnet URI against an existing
/// want. The one-of and want-exists rules are enforced by the bridge handler,
/// not this DTO, matching today's layering.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct EnqueueDownloadParams {
    /// A release_id from a prior harmonia_search_releases result.
    pub release_id: Option<String>,
    /// A magnet: URI. Raw http(s) URLs are rejected — use release_id for credentialed indexer results.
    pub magnet: Option<String>,
    /// Queue priority 1 (highest) to 4 (lowest); out-of-range values are clamped by the bridge.
    #[serde(default = "default_enqueue_priority")]
    #[schemars(range(min = 1, max = 4))]
    pub priority: u8,
    /// An existing want row UUID to associate this download with — must already exist; the tool never invents one.
    pub want_id: Option<String>,
}

/// Parameters for `harmonia_list_downloads` — lists `download_queue` rows,
/// optionally filtered.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct ListDownloadsParams {
    /// Filter by queue status — validated by the bridge against the six known statuses.
    pub status: Option<String>,
    /// Filter to one download_queue row id.
    pub id: Option<String>,
    /// Maximum rows to return; the bridge clamps to 1..=500.
    #[serde(default = "default_list_limit")]
    pub limit: u32,
}

/// Parameters for `harmonia_cancel_download` — cancels a queued or active
/// download, stopping a live transfer when one is active.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub struct CancelDownloadParams {
    /// The download_queue row id (from harmonia_enqueue_download or harmonia_list_downloads).
    pub id: String, // kanon:ignore RUST/primitive-for-domain-id -- MCP wire DTO mirroring the bridge's own `CancelArguments { id: String }`; UUID validation stays a bridge-level tool error today
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn parse<T: serde::de::DeserializeOwned>(arguments: Value) -> Result<T, serde_json::Error> {
        serde_json::from_value(arguments)
    }

    // ── harmonia_db_migrate ────────────────────────────────────────────────

    #[test]
    fn db_migrate_accepts_what_the_live_extractor_accepts() {
        // No arguments at all -> consumer-side default "harmonia.toml".
        let params: DbMigrateParams = parse(json!({})).unwrap();
        assert_eq!(params.config, None);

        let params: DbMigrateParams = parse(json!({ "config": "custom/harmonia.toml" })).unwrap();
        assert_eq!(params.config, Some(PathBuf::from("custom/harmonia.toml")));

        // The hand-rolled optional_string maps explicit null to absent.
        let params: DbMigrateParams = parse(json!({ "config": null })).unwrap();
        assert_eq!(params.config, None);

        // Unknown keys are ignored today (the advertised
        // additionalProperties:false was never enforced by the parser).
        let params: DbMigrateParams = parse(json!({ "config": "x.toml", "bogus": 1 })).unwrap();
        assert_eq!(params.config, Some(PathBuf::from("x.toml")));
    }

    #[test]
    fn db_migrate_rejects_a_non_string_config() {
        assert!(parse::<DbMigrateParams>(json!({ "config": 42 })).is_err());
        assert!(parse::<DbMigrateParams>(json!({ "config": true })).is_err());
        assert!(parse::<DbMigrateParams>(json!({ "config": ["x.toml"] })).is_err());
    }

    // ── harmonia_migrate_library ───────────────────────────────────────────

    #[test]
    fn migrate_library_accepts_what_the_live_extractor_accepts() {
        let params: MigrateLibraryParams = parse(json!({
            "source": "/legacy/in",
            "target": "/library/out",
            "media_type": "music"
        }))
        .unwrap();
        assert_eq!(params.source, PathBuf::from("/legacy/in"));
        assert_eq!(params.target, PathBuf::from("/library/out"));
        assert_eq!(params.media_type, MigrateMediaType::Music);
        // Missing flags default to false, as optional_bool does today.
        assert!(!params.dry_run);
        assert!(!params.copy);

        for (raw, expected) in [
            ("music", MigrateMediaType::Music),
            ("books", MigrateMediaType::Books),
            ("audiobooks", MigrateMediaType::Audiobooks),
            ("podcasts", MigrateMediaType::Podcasts),
        ] {
            let params: MigrateLibraryParams = parse(json!({
                "source": "/a",
                "target": "/b",
                "media_type": raw
            }))
            .unwrap();
            assert_eq!(params.media_type, expected);
        }

        let params: MigrateLibraryParams = parse(json!({
            "source": "/a",
            "target": "/b",
            "media_type": "books",
            "dry_run": true,
            "copy": true,
            "ignored_unknown_key": "present today"
        }))
        .unwrap();
        assert!(params.dry_run);
        assert!(params.copy);
    }

    #[test]
    fn migrate_library_bool_flags_treat_null_as_false_like_the_live_extractor() {
        let params: MigrateLibraryParams = parse(json!({
            "source": "/a",
            "target": "/b",
            "media_type": "music",
            "dry_run": null,
            "copy": null
        }))
        .unwrap();
        assert!(!params.dry_run);
        assert!(!params.copy);

        // A non-boolean flag is rejected by both the live extractor and serde.
        assert!(
            parse::<MigrateLibraryParams>(json!({
                "source": "/a",
                "target": "/b",
                "media_type": "music",
                "dry_run": "yes"
            }))
            .is_err()
        );
    }

    #[test]
    fn migrate_library_rejects_missing_or_invalid_required_arguments() {
        // Missing required keys.
        assert!(
            parse::<MigrateLibraryParams>(json!({ "target": "/b", "media_type": "music" }))
                .is_err()
        );
        assert!(
            parse::<MigrateLibraryParams>(json!({ "source": "/a", "media_type": "music" }))
                .is_err()
        );
        assert!(parse::<MigrateLibraryParams>(json!({ "source": "/a", "target": "/b" })).is_err());
        // Explicit null on a required key is rejected (live: "missing required argument").
        assert!(
            parse::<MigrateLibraryParams>(
                json!({ "source": null, "target": "/b", "media_type": "music" })
            )
            .is_err()
        );
        // A media_type outside the closed set.
        assert!(
            parse::<MigrateLibraryParams>(
                json!({ "source": "/a", "target": "/b", "media_type": "video" })
            )
            .is_err()
        );
        // Wrong types on required keys.
        assert!(
            parse::<MigrateLibraryParams>(
                json!({ "source": 1, "target": "/b", "media_type": "music" })
            )
            .is_err()
        );
    }

    // ── harmonia_play_file ─────────────────────────────────────────────────

    #[test]
    fn play_file_accepts_what_the_live_extractor_accepts() {
        let params: PlayFileParams = parse(json!({ "file": "/music/song.flac" })).unwrap();
        assert_eq!(params.file, PathBuf::from("/music/song.flac"));
        assert_eq!(params.device, None);

        let params: PlayFileParams =
            parse(json!({ "file": "/x.flac", "device": "USB DAC", "unknown": true })).unwrap();
        assert_eq!(params.device.as_deref(), Some("USB DAC"));

        // Explicit null on the optional device is absent today.
        let params: PlayFileParams = parse(json!({ "file": "/x.flac", "device": null })).unwrap();
        assert_eq!(params.device, None);
    }

    #[test]
    fn play_file_rejects_missing_file_and_wrong_types() {
        assert!(parse::<PlayFileParams>(json!({})).is_err());
        assert!(parse::<PlayFileParams>(json!({ "file": null })).is_err());
        assert!(parse::<PlayFileParams>(json!({ "file": 7 })).is_err());
        assert!(parse::<PlayFileParams>(json!({ "file": "/x.flac", "device": 3 })).is_err());
    }

    // ── harmonia_render ────────────────────────────────────────────────────

    #[test]
    fn render_accepts_what_the_live_extractor_accepts() {
        // Every field is optional.
        let params: RenderParams = parse(json!({})).unwrap();
        assert_eq!(params.server, None);
        assert_eq!(params.cert_dir, None);
        assert_eq!(params.name, None);
        assert_eq!(params.config, None);

        let params: RenderParams = parse(json!({
            "server": "192.0.2.10:8999",
            "cert_dir": "/etc/harmonia/renderer",
            "name": "living-room",
            "config": "/etc/harmonia/renderer.toml"
        }))
        .unwrap();
        assert_eq!(params.server, Some("192.0.2.10:8999".parse().unwrap()));
        assert_eq!(
            params.cert_dir,
            Some(PathBuf::from("/etc/harmonia/renderer"))
        );
        assert_eq!(params.name.as_deref(), Some("living-room"));
        assert_eq!(
            params.config,
            Some(PathBuf::from("/etc/harmonia/renderer.toml"))
        );

        // Explicit null on optional fields is absent today.
        let params: RenderParams = parse(json!({ "server": null })).unwrap();
        assert_eq!(params.server, None);
    }

    #[test]
    fn render_rejects_an_unparseable_server_address_like_the_live_extractor() {
        // Live: optional_socket_addr parses the string as a SocketAddr.
        assert!(parse::<RenderParams>(json!({ "server": "not-an-address" })).is_err());
        assert!(parse::<RenderParams>(json!({ "server": "localhost:8999" })).is_err());
        assert!(parse::<RenderParams>(json!({ "server": 8999 })).is_err());
        assert!(parse::<RenderParams>(json!({ "cert_dir": 42 })).is_err());
    }

    // ── Lifecycle status/stop params (play_file and render trios) ──────────

    #[test]
    fn lifecycle_status_and_stop_params_accept_an_optional_op_id() {
        // Every status/stop DTO takes at most one optional string op_id;
        // empty and explicit-null arguments both mean "the current op".
        let status: PlayFileStatusParams = parse(json!({})).unwrap();
        assert_eq!(status.op_id, None);
        let status: PlayFileStatusParams = parse(json!({ "op_id": null })).unwrap();
        assert_eq!(status.op_id, None);
        let status: PlayFileStatusParams = parse(json!({ "op_id": "playback-1" })).unwrap();
        assert_eq!(status.op_id.as_deref(), Some("playback-1"));

        let stop: PlayFileStopParams = parse(json!({})).unwrap();
        assert_eq!(stop.op_id, None);
        let stop: PlayFileStopParams = parse(json!({ "op_id": "playback-2" })).unwrap();
        assert_eq!(stop.op_id.as_deref(), Some("playback-2"));

        let status: RenderStatusParams = parse(json!({})).unwrap();
        assert_eq!(status.op_id, None);
        let status: RenderStatusParams = parse(json!({ "op_id": "renderer-1" })).unwrap();
        assert_eq!(status.op_id.as_deref(), Some("renderer-1"));

        let stop: RenderStopParams = parse(json!({})).unwrap();
        assert_eq!(stop.op_id, None);
        let stop: RenderStopParams = parse(json!({ "op_id": "renderer-3" })).unwrap();
        assert_eq!(stop.op_id.as_deref(), Some("renderer-3"));
    }

    #[test]
    fn lifecycle_status_and_stop_params_reject_a_non_string_op_id() {
        assert!(parse::<PlayFileStatusParams>(json!({ "op_id": 7 })).is_err());
        assert!(parse::<PlayFileStatusParams>(json!({ "op_id": ["playback-1"] })).is_err());
        assert!(parse::<PlayFileStopParams>(json!({ "op_id": true })).is_err());
        assert!(parse::<RenderStatusParams>(json!({ "op_id": 1.5 })).is_err());
        assert!(parse::<RenderStopParams>(json!({ "op_id": { "id": "x" } })).is_err());
    }

    // ── harmonia_search_releases ───────────────────────────────────────────

    #[test]
    fn search_releases_accepts_what_the_bridge_accepts() {
        let params: SearchReleasesParams = parse(json!({})).unwrap();
        assert_eq!(params.limit, 100);
        assert_eq!(params.offset, 0);
        assert!(params.category_ids.is_empty());
        assert_eq!(params.query_text, None);

        let params: SearchReleasesParams = parse(json!({
            "query_text": "in rainbows",
            "media_type": "music",
            "artist": "radiohead",
            "album": "in rainbows",
            "author": null,
            "imdb_id": "tt0421715",
            "tvdb_id": 12345,
            "tmdb_id": 670,
            "season": 2,
            "episode": 5,
            "category_ids": [2000, 3000],
            "limit": 25,
            "offset": 50,
            "unknown_key": "ignored today"
        }))
        .unwrap();
        assert_eq!(params.query_text.as_deref(), Some("in rainbows"));
        assert_eq!(params.author, None);
        assert_eq!(params.tvdb_id, Some(12345));
        assert_eq!(params.category_ids, vec![2000, 3000]);
        assert_eq!(params.limit, 25);
        assert_eq!(params.offset, 50);

        // DRIFT PINNED: the advertised enum (any/tv/movie/music/book) is not
        // validated at the DTO layer — SearchRequest takes any string and the
        // search service decides. Match the live surface, not the schema.
        let params: SearchReleasesParams = parse(json!({ "media_type": "song" })).unwrap();
        assert_eq!(params.media_type.as_deref(), Some("song"));
    }

    #[test]
    fn search_releases_rejects_what_the_bridge_rejects() {
        // serde(default) covers a MISSING key, not an explicit null — the
        // bridge's SearchRequest rejects all of these today.
        assert!(parse::<SearchReleasesParams>(json!({ "limit": null })).is_err());
        assert!(parse::<SearchReleasesParams>(json!({ "offset": null })).is_err());
        assert!(parse::<SearchReleasesParams>(json!({ "category_ids": null })).is_err());
        // u32 fields reject negatives, floats, and strings.
        assert!(parse::<SearchReleasesParams>(json!({ "limit": -1 })).is_err());
        assert!(parse::<SearchReleasesParams>(json!({ "limit": "100" })).is_err());
        assert!(parse::<SearchReleasesParams>(json!({ "tvdb_id": "12345" })).is_err());
        assert!(parse::<SearchReleasesParams>(json!({ "season": 2.5 })).is_err());
        assert!(parse::<SearchReleasesParams>(json!({ "query_text": 7 })).is_err());
    }

    #[test]
    fn search_releases_round_trips_into_the_bridges_own_dto() {
        // WHY: PR 2 re-serializes these params to forward over the bridge,
        // which re-parses them as `SearchRequest` — prove that round trip is
        // lossless for a fully-populated call (memo §6 bridge-coupling risk).
        let params: SearchReleasesParams = parse(json!({
            "query_text": "in rainbows",
            "media_type": "music",
            "category_ids": [2000],
            "limit": 25
        }))
        .unwrap();
        let forwarded = serde_json::to_value(&params).unwrap();
        let bridge_view: paroche::routes::search::SearchRequest =
            serde_json::from_value(forwarded).unwrap();
        assert_eq!(bridge_view.query_text.as_deref(), Some("in rainbows"));
        assert_eq!(bridge_view.media_type.as_deref(), Some("music"));
        assert_eq!(bridge_view.category_ids, vec![2000]);
        assert_eq!(bridge_view.limit, 25);
    }

    // ── harmonia_enqueue_download ──────────────────────────────────────────

    #[test]
    fn enqueue_download_accepts_what_the_bridge_accepts() {
        let params: EnqueueDownloadParams = parse(json!({
            "magnet": "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567",
            "want_id": "0191b2c3-7a2f-7c3d-8e4f-9a0b1c2d3e4f"
        }))
        .unwrap();
        assert_eq!(
            params.priority, 4,
            "default priority matches the bridge DTO"
        );
        assert_eq!(params.release_id, None);

        // DRIFT PINNED: out-of-range priorities deserialize (u8) and the
        // bridge clamps to 1..=4 — the schema's minimum/maximum is not a
        // parser rejection today.
        let params: EnqueueDownloadParams = parse(json!({ "priority": 0 })).unwrap();
        assert_eq!(params.priority, 0);
        let params: EnqueueDownloadParams = parse(json!({ "priority": 255 })).unwrap();
        assert_eq!(params.priority, 255);

        // DRIFT PINNED: both-arms and missing-want_id calls DESERIALIZE — the
        // rejections ("exactly one of release_id or magnet", "want_id is
        // required") are bridge-level tool errors, not parser errors.
        let params: EnqueueDownloadParams =
            parse(json!({ "release_id": "r", "magnet": "m" })).unwrap();
        assert_eq!(params.release_id.as_deref(), Some("r"));
        let params: EnqueueDownloadParams = parse(json!({})).unwrap();
        assert_eq!(params.want_id, None);
    }

    #[test]
    fn enqueue_download_rejects_what_the_bridge_rejects() {
        // serde(default) does not cover explicit null.
        assert!(parse::<EnqueueDownloadParams>(json!({ "priority": null })).is_err());
        // u8 range/type checks.
        assert!(parse::<EnqueueDownloadParams>(json!({ "priority": 256 })).is_err());
        assert!(parse::<EnqueueDownloadParams>(json!({ "priority": -1 })).is_err());
        assert!(parse::<EnqueueDownloadParams>(json!({ "priority": 2.5 })).is_err());
        assert!(parse::<EnqueueDownloadParams>(json!({ "priority": "high" })).is_err());
        assert!(parse::<EnqueueDownloadParams>(json!({ "release_id": 42 })).is_err());
        assert!(parse::<EnqueueDownloadParams>(json!({ "magnet": false })).is_err());
    }

    // ── harmonia_list_downloads ────────────────────────────────────────────

    #[test]
    fn list_downloads_accepts_what_the_bridge_accepts() {
        let params: ListDownloadsParams = parse(json!({})).unwrap();
        assert_eq!(params.limit, 50);
        assert_eq!(params.status, None);
        assert_eq!(params.id, None);

        let params: ListDownloadsParams =
            parse(json!({ "status": "queued", "id": "some-id", "limit": 10 })).unwrap();
        assert_eq!(params.status.as_deref(), Some("queued"));
        assert_eq!(params.limit, 10);

        // DRIFT PINNED: the advertised status enum is validated by the bridge
        // handler (tool error), not at the DTO layer; an unknown status
        // string still deserializes.
        let params: ListDownloadsParams = parse(json!({ "status": "bogus" })).unwrap();
        assert_eq!(params.status.as_deref(), Some("bogus"));

        // Out-of-range limits deserialize; the bridge clamps to 1..=500.
        let params: ListDownloadsParams = parse(json!({ "limit": 9999 })).unwrap();
        assert_eq!(params.limit, 9999);
    }

    #[test]
    fn list_downloads_rejects_what_the_bridge_rejects() {
        assert!(parse::<ListDownloadsParams>(json!({ "limit": null })).is_err());
        assert!(parse::<ListDownloadsParams>(json!({ "limit": -5 })).is_err());
        assert!(parse::<ListDownloadsParams>(json!({ "limit": "50" })).is_err());
        assert!(parse::<ListDownloadsParams>(json!({ "status": 7 })).is_err());
        assert!(parse::<ListDownloadsParams>(json!({ "id": 42 })).is_err());
    }

    // ── harmonia_cancel_download ───────────────────────────────────────────

    #[test]
    fn cancel_download_accepts_what_the_bridge_accepts() {
        let params: CancelDownloadParams = parse(json!({ "id": "queue-row-id" })).unwrap();
        assert_eq!(params.id, "queue-row-id");
        // UUID validity is a bridge-level check; any string deserializes.
        let params: CancelDownloadParams = parse(json!({ "id": "not-a-uuid" })).unwrap();
        assert_eq!(params.id, "not-a-uuid");
    }

    #[test]
    fn cancel_download_rejects_a_missing_null_or_non_string_id() {
        assert!(parse::<CancelDownloadParams>(json!({})).is_err());
        assert!(parse::<CancelDownloadParams>(json!({ "id": null })).is_err());
        assert!(parse::<CancelDownloadParams>(json!({ "id": 42 })).is_err());
    }

    // ── Bridge-forwarding round trip (acquisition tools) ───────────────────

    #[test]
    fn acquisition_params_survive_a_serialize_reparse_round_trip() {
        // WHY: PR 2 forwards typed acquisition params to the bridge as JSON —
        // serialize then re-parse must be identity for each of the 4 DTOs.
        let search: SearchReleasesParams = parse(json!({ "query_text": "q", "limit": 5 })).unwrap();
        let again: SearchReleasesParams = parse(serde_json::to_value(&search).unwrap()).unwrap();
        assert_eq!(search, again);

        let enqueue: EnqueueDownloadParams =
            parse(json!({ "magnet": "m", "want_id": "w" })).unwrap();
        let again: EnqueueDownloadParams = parse(serde_json::to_value(&enqueue).unwrap()).unwrap();
        assert_eq!(enqueue, again);

        let list: ListDownloadsParams = parse(json!({ "status": "failed" })).unwrap();
        let again: ListDownloadsParams = parse(serde_json::to_value(&list).unwrap()).unwrap();
        assert_eq!(list, again);

        let cancel: CancelDownloadParams = parse(json!({ "id": "x" })).unwrap();
        let again: CancelDownloadParams = parse(serde_json::to_value(&cancel).unwrap()).unwrap();
        assert_eq!(cancel, again);
    }

    // ── Schemars schema smoke checks (the PR-2 advertised contract) ────────

    #[test]
    fn generated_schemas_carry_todays_required_sets_and_defaults() {
        let play = serde_json::to_value(schemars::schema_for!(PlayFileParams)).unwrap();
        assert_eq!(play.pointer("/required/0"), Some(&json!("file")));
        assert_eq!(
            play.pointer("/properties/file/description"),
            Some(&json!("Path to the audio file.")),
            "field doc comments become schema descriptions: {play}"
        );

        let migrate = serde_json::to_value(schemars::schema_for!(MigrateLibraryParams)).unwrap();
        for key in ["source", "target", "media_type"] {
            assert!(
                migrate
                    .pointer("/required")
                    .and_then(Value::as_array)
                    .unwrap()
                    .contains(&json!(key)),
                "{key} must stay required: {migrate}"
            );
        }
        // WHY oneOf+const, not a bare enum array: schemars 1.x emits each
        // documented variant as its own subschema carrying the doc comment.
        let consts: Vec<Value> = migrate
            .pointer("/$defs/MigrateMediaType/oneOf")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .map(|variant| variant["const"].clone())
            .collect();
        assert_eq!(
            consts,
            vec![
                json!("music"),
                json!("books"),
                json!("audiobooks"),
                json!("podcasts")
            ],
            "{migrate}"
        );
        assert_eq!(
            migrate.pointer("/properties/dry_run/default"),
            Some(&json!(false)),
            "{migrate}"
        );

        let enqueue = serde_json::to_value(schemars::schema_for!(EnqueueDownloadParams)).unwrap();
        assert_eq!(
            enqueue.pointer("/properties/priority/default"),
            Some(&json!(4)),
            "{enqueue}"
        );
        assert_eq!(
            enqueue.pointer("/properties/priority/minimum"),
            Some(&json!(1)),
            "{enqueue}"
        );
        assert_eq!(
            enqueue.pointer("/properties/priority/maximum"),
            Some(&json!(4)),
            "{enqueue}"
        );

        // Schema generation must not panic for any of the 12 DTOs.
        let _ = schemars::schema_for!(DbMigrateParams);
        let _ = schemars::schema_for!(RenderParams);
        let _ = schemars::schema_for!(SearchReleasesParams);
        let _ = schemars::schema_for!(ListDownloadsParams);
        let _ = schemars::schema_for!(CancelDownloadParams);
        let _ = schemars::schema_for!(PlayFileStatusParams);
        let _ = schemars::schema_for!(PlayFileStopParams);
        let _ = schemars::schema_for!(RenderStatusParams);
        let _ = schemars::schema_for!(RenderStopParams);
    }
}
