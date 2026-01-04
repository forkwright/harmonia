// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.TV;

public class Episode : MediaItem
{
    public Episode()
    {
        MediaType = MediaType.TV;
    }

    public int SeriesId { get; set; }
    public int SeasonNumber { get; set; }
    public int EpisodeNumber { get; set; }
    public int? AbsoluteEpisodeNumber { get; set; }

    public int? SceneSeasonNumber { get; set; }
    public int? SceneEpisodeNumber { get; set; }
    public int? SceneAbsoluteEpisodeNumber { get; set; }

    public string? Title { get; set; }
    public string? Overview { get; set; }
    public DateTime? AirDate { get; set; }
    public DateTime? AirDateUtc { get; set; }
    public int? EpisodeFileId { get; set; }

    public override string GetTitle() => Title ?? $"Episode {EpisodeNumber}";
    public override int GetYear() => AirDate?.Year ?? 0;

    public override string ToString()
    {
        return $"S{SeasonNumber:00}E{EpisodeNumber:00} - {Title}";
    }
}
