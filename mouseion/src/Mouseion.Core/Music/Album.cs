// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Music;

public class Album : ModelBase
{
    public Album()
    {
        Images = new List<string>();
        Genres = new List<string>();
        Tags = new HashSet<int>();
        MediaType = MediaType.Music;
    }

    public int? ArtistId { get; set; }
    public string Title { get; set; } = null!;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public string? ForeignAlbumId { get; set; }
    public string? DiscogsId { get; set; }
    public string? MusicBrainzId { get; set; }
    public string? ReleaseGroupMbid { get; set; }
    public string? ReleaseStatus { get; set; }
    public string? ReleaseCountry { get; set; }
    public string? RecordLabel { get; set; }
    public MediaType MediaType { get; set; }
    public DateTime? ReleaseDate { get; set; }
    public string? AlbumType { get; set; }
    public List<string> Images { get; set; }
    public decimal? Rating { get; set; }
    public int? Votes { get; set; }
    public List<string> Genres { get; set; }
    public int? TrackCount { get; set; }
    public int? DiscCount { get; set; }
    public int? Duration { get; set; }
    public bool Monitored { get; set; }
    public int QualityProfileId { get; set; }
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public DateTime Added { get; set; }
    public HashSet<int> Tags { get; set; }
    public DateTime? LastSearchTime { get; set; }

    public override string ToString() => $"{Title} ({ReleaseDate?.Year})";
}
