// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.MediaItems;

public abstract class MediaItem : ModelBase
{
    protected MediaItem()
    {
        Tags = new HashSet<int>();
        Added = DateTime.UtcNow;
    }

    public MediaType MediaType { get; set; }
    public bool Monitored { get; set; }
    public int QualityProfileId { get; set; }
    public string Path { get; set; } = string.Empty;
    public string RootFolderPath { get; set; } = string.Empty;
    public DateTime Added { get; set; }
    public HashSet<int> Tags { get; set; }
    public DateTime? LastSearchTime { get; set; }

    // Foreign keys for hierarchical parent entities
    public int? AuthorId { get; set; }      // For Books/Audiobooks
    public int? ArtistId { get; set; }      // For Music (Albums/Tracks)
    public int? TVShowId { get; set; }      // For TV Episodes
    public int? BookSeriesId { get; set; }  // For Books in a series

    // Abstract methods for polymorphic behavior
    public abstract string GetTitle();
    public abstract int GetYear();
}
