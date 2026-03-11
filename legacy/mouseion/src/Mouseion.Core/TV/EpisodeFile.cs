// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.TV;

public class EpisodeFile : ModelBase
{
    public int SeriesId { get; set; }
    public int SeasonNumber { get; set; }
    public string RelativePath { get; set; } = null!;
    public long Size { get; set; }
    public DateTime DateAdded { get; set; } = DateTime.UtcNow;
    public string? SceneName { get; set; }
    public string? ReleaseGroup { get; set; }
    public string? Quality { get; set; }  // JSON serialized quality info
    public string? MediaInfo { get; set; }  // JSON serialized media info

    public override string ToString()
    {
        return RelativePath;
    }
}
