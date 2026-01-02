// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Music;

public class Track : MediaItem
{
    public Track()
    {
        MediaType = MediaType.Music;
    }

    public int? AlbumId { get; set; }
    public new int? ArtistId { get; set; }
    public string Title { get; set; } = null!;
    public string? ForeignTrackId { get; set; }
    public string? MusicBrainzId { get; set; }
    public int TrackNumber { get; set; }
    public int DiscNumber { get; set; } = 1;
    public int? DurationSeconds { get; set; }
    public bool Explicit { get; set; }

    public override string GetTitle() => Title;
    public override int GetYear() => 0;

    public override string ToString() => $"{DiscNumber}-{TrackNumber}: {Title}";
}
