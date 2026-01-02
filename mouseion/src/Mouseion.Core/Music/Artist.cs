// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Music;

public class Artist : ModelBase
{
    public Artist()
    {
        Images = new List<string>();
        Genres = new List<string>();
        Tags = new HashSet<int>();
    }

    public string Name { get; set; } = null!;
    public string? SortName { get; set; }
    public string? Description { get; set; }
    public string? ForeignArtistId { get; set; }
    public string? DiscogsId { get; set; }
    public string? MusicBrainzId { get; set; }
    public string? SpotifyId { get; set; }
    public string? LastFmId { get; set; }
    public string? ArtistType { get; set; }
    public string? Status { get; set; }
    public List<string> Images { get; set; }
    public decimal? Rating { get; set; }
    public int? Votes { get; set; }
    public List<string> Genres { get; set; }
    public string? Country { get; set; }
    public DateTime? BeginDate { get; set; }
    public DateTime? EndDate { get; set; }
    public bool Monitored { get; set; }
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; }
    public DateTime Added { get; set; }
    public HashSet<int> Tags { get; set; }

    public override string ToString() => Name;
}
