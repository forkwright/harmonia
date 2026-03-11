// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.SmartLists.Sources;

/// <summary>
/// Interface for external discovery sources. Each source queries a public API
/// and returns normalized discovery results that can be added to the library.
/// </summary>
public interface ISmartListSourceProvider
{
    SmartListSource Source { get; }

    /// <summary>
    /// Query the external source and return discovery results.
    /// </summary>
    Task<IReadOnlyList<SmartListDiscoveryResult>> DiscoverAsync(
        SmartList smartList,
        CancellationToken ct = default);
}

/// <summary>
/// Normalized result from any discovery source. Contains enough information
/// to deduplicate against the library and add if new.
/// </summary>
public class SmartListDiscoveryResult
{
    /// <summary>Source-specific external ID (e.g., TMDB ID, AniList ID).</summary>
    public string ExternalId { get; set; } = string.Empty;

    public string Title { get; set; } = string.Empty;
    public int Year { get; set; }

    /// <summary>Normalized rating 0-100.</summary>
    public int? Rating { get; set; }

    /// <summary>Comma-separated genre names.</summary>
    public string? Genres { get; set; }

    /// <summary>Overview/description.</summary>
    public string? Overview { get; set; }

    /// <summary>Poster/cover URL.</summary>
    public string? PosterUrl { get; set; }

    // Cross-reference IDs for library dedup
    public int? TmdbId { get; set; }
    public string? ImdbId { get; set; }
    public int? TvdbId { get; set; }
    public int? AniListId { get; set; }
    public int? MalId { get; set; }
    public Guid? MusicBrainzId { get; set; }
    public string? Isbn { get; set; }

    /// <summary>Full source-specific metadata for later use.</summary>
    public string? MetadataJson { get; set; }
}
