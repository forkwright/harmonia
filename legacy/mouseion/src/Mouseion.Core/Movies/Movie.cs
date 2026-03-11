// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Movies;

public class Movie : MediaItem
{
    public Movie()
    {
        MediaType = MediaType.Movie;
        Images = new List<string>();
        Genres = new List<string>();
        Tags = new HashSet<int>();
    }

    public string Title { get; set; } = null!;
    public int Year { get; set; }
    public string? Overview { get; set; }
    public int? Runtime { get; set; }
    public string? TmdbId { get; set; }
    public string? ImdbId { get; set; }
    public List<string> Images { get; set; }
    public List<string> Genres { get; set; }
    public DateTime? InCinemas { get; set; }
    public DateTime? PhysicalRelease { get; set; }
    public DateTime? DigitalRelease { get; set; }
    public string? Certification { get; set; }
    public string? Studio { get; set; }
    public string? Website { get; set; }
    public string? YouTubeTrailerId { get; set; }
    public float? Popularity { get; set; }
    public int? CollectionId { get; set; }

    public override string GetTitle() => Title;
    public override int GetYear() => Year;
}
