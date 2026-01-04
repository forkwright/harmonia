// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.MediaCovers;

public enum MediaCoverType
{
    Unknown = 0,
    Poster = 1,
    Banner = 2,
    Fanart = 3,
    Screenshot = 4,
    Headshot = 5,
    Clearlogo = 6
}

public class MediaCover
{
    public MediaCoverType CoverType { get; set; }
    public string Url { get; set; } = string.Empty;
    public string RemoteUrl { get; set; } = string.Empty;

    public MediaCover()
    {
    }

    public MediaCover(MediaCoverType coverType, string remoteUrl)
    {
        CoverType = coverType;
        RemoteUrl = remoteUrl;
    }

    public override bool Equals(object? obj)
    {
        if (obj is not MediaCover other)
        {
            return false;
        }

        return CoverType == other.CoverType && RemoteUrl == other.RemoteUrl;
    }

    public override int GetHashCode()
    {
        return HashCode.Combine(CoverType, RemoteUrl);
    }
}
