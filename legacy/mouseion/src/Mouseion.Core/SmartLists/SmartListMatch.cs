// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.SmartLists;

/// <summary>
/// A discovered item from a Smart List source, pending or completed addition to the library.
/// Tracks what was found, whether it was added, and dedup state.
/// </summary>
public class SmartListMatch : ModelBase
{
    public int SmartListId { get; set; }

    public string ExternalId { get; set; } = string.Empty;

    public MediaType MediaType { get; set; }

    public string Title { get; set; } = string.Empty;

    public int Year { get; set; }

    /// <summary>
    /// Normalized rating 0-100 from the source.
    /// </summary>
    public int? Rating { get; set; }

    /// <summary>
    /// Whether this match was added to the library.
    /// </summary>
    public SmartListMatchStatus Status { get; set; } = SmartListMatchStatus.Pending;

    /// <summary>
    /// The Mouseion MediaItem ID if this was added.
    /// </summary>
    public int? MediaItemId { get; set; }

    /// <summary>
    /// Source-specific metadata JSON (poster URL, overview, genres, etc.).
    /// </summary>
    public string? MetadataJson { get; set; }

    public DateTime DiscoveredAt { get; set; }

    public DateTime? AddedAt { get; set; }
}

public enum SmartListMatchStatus
{
    /// <summary>Discovered but not yet processed.</summary>
    Pending = 0,

    /// <summary>Successfully added to library.</summary>
    Added = 1,

    /// <summary>Already exists in library (dedup hit).</summary>
    Duplicate = 2,

    /// <summary>Filtered out (below minimum rating, excluded genre, etc.).</summary>
    Filtered = 3,

    /// <summary>User manually skipped/rejected.</summary>
    Skipped = 4,

    /// <summary>Failed to add (metadata lookup error, etc.).</summary>
    Failed = 5
}
