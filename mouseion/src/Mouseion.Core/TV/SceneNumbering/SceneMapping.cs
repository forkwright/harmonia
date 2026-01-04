// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.TV.SceneNumbering;

public class SceneMapping : ModelBase
{
    public int TvdbId { get; set; }
    public int? SeasonNumber { get; set; }
    public int? SceneSeasonNumber { get; set; }
    public int? EpisodeNumber { get; set; }
    public int? SceneEpisodeNumber { get; set; }
    public string? Title { get; set; }

    public override string ToString()
    {
        if (SeasonNumber.HasValue && EpisodeNumber.HasValue)
        {
            return $"TVDB: S{SeasonNumber:00}E{EpisodeNumber:00} → Scene: S{SceneSeasonNumber:00}E{SceneEpisodeNumber:00}";
        }
        return $"TVDB ID: {TvdbId}";
    }
}
