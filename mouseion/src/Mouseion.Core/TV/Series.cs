// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.TV;

public class Series : ModelBase
{
    public Series()
    {
        Tags = new HashSet<int>();
        Genres = new List<string>();
        Images = new List<string>();
    }

    public int? TvdbId { get; set; }
    public int? TmdbId { get; set; }
    public string? ImdbId { get; set; }

    public string Title { get; set; } = null!;
    public string? SortTitle { get; set; }
    public string? CleanTitle { get; set; }
    public string? Overview { get; set; }
    public string Status { get; set; } = "Continuing";
    public string? AirTime { get; set; }
    public string? Network { get; set; }
    public int? Runtime { get; set; }
    public List<string> Genres { get; set; }
    public int Year { get; set; }
    public DateTime? FirstAired { get; set; }
    public List<string> Images { get; set; }

    public string Path { get; set; } = null!;
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; }
    public bool SeasonFolder { get; set; } = true;
    public bool Monitored { get; set; }
    public bool UseSceneNumbering { get; set; }
    public DateTime Added { get; set; } = DateTime.UtcNow;
    public HashSet<int> Tags { get; set; }

    public override string ToString()
    {
        return $"{Title} ({Year})";
    }
}
