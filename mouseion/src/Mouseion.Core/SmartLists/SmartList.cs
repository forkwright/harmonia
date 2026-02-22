// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.SmartLists;

/// <summary>
/// A discovery-driven list that auto-adds items from external metadata sources.
/// Unlike ImportLists (which sync a user's personal collection), Smart Lists query
/// public discovery APIs with filters (trending, genre, rating thresholds, etc.).
/// </summary>
public class SmartList : ModelBase
{
    public string Name { get; set; } = string.Empty;

    /// <summary>
    /// Which external source to query (TMDB, Trakt, AniList, MusicBrainz, OpenLibrary).
    /// </summary>
    public SmartListSource Source { get; set; }

    /// <summary>
    /// Target media type for discovered items.
    /// </summary>
    public MediaType MediaType { get; set; }

    /// <summary>
    /// Serialized JSON of source-specific query parameters (genre, year range, keywords, etc.).
    /// </summary>
    public string QueryParametersJson { get; set; } = "{}";

    /// <summary>
    /// Quality profile to assign to auto-added items.
    /// </summary>
    public int QualityProfileId { get; set; }

    /// <summary>
    /// Root folder for auto-added items.
    /// </summary>
    public string RootFolderPath { get; set; } = string.Empty;

    /// <summary>
    /// How often to re-query the source for new matches.
    /// </summary>
    public SmartListRefreshInterval RefreshInterval { get; set; } = SmartListRefreshInterval.Weekly;

    /// <summary>
    /// Whether to automatically trigger indexer search when items are added.
    /// </summary>
    public bool SearchOnAdd { get; set; }

    /// <summary>
    /// Whether this list is actively refreshing on schedule.
    /// </summary>
    public bool Enabled { get; set; } = true;

    /// <summary>
    /// Maximum number of items to add per refresh cycle (prevents flooding).
    /// </summary>
    public int MaxItemsPerRefresh { get; set; } = 50;

    /// <summary>
    /// Minimum rating threshold (0-100 normalized). Items below this are skipped.
    /// </summary>
    public int? MinimumRating { get; set; }

    /// <summary>
    /// Minimum year filter. Null = no lower bound.
    /// </summary>
    public int? MinYear { get; set; }

    /// <summary>
    /// Maximum year filter. Null = no upper bound.
    /// </summary>
    public int? MaxYear { get; set; }

    /// <summary>
    /// Comma-separated genre IDs to exclude.
    /// </summary>
    public string? ExcludeGenres { get; set; }

    /// <summary>
    /// Language filter (ISO 639-1 code, e.g. "en", "ja").
    /// </summary>
    public string? Language { get; set; }

    /// <summary>
    /// Tags for organizational grouping.
    /// </summary>
    public string? Tags { get; set; }

    public int ItemsAdded { get; set; }

    public DateTime? LastRefreshed { get; set; }

    public DateTime CreatedAt { get; set; }

    public DateTime UpdatedAt { get; set; }
}

public enum SmartListSource
{
    /// <summary>TMDB Discover API — movies and TV shows by genre, year, rating, keywords, popularity.</summary>
    TmdbDiscover = 1,

    /// <summary>Trakt public lists — trending, popular, anticipated, box office.</summary>
    TraktPublic = 2,

    /// <summary>AniList discovery — anime and manga by genre, score, season, popularity.</summary>
    AniListDiscover = 3,

    /// <summary>MusicBrainz — new releases by tag, area, artist type.</summary>
    MusicBrainzReleases = 4,

    /// <summary>OpenLibrary — books by subject, list, author.</summary>
    OpenLibrarySubject = 5
}

public enum SmartListRefreshInterval
{
    Daily = 1,
    Weekly = 2,
    Biweekly = 3,
    Monthly = 4
}
