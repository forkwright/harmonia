// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.Music;

public class MusicFile : ModelBase
{
    public int? TrackId { get; set; }
    public int? AlbumId { get; set; }
    public string? RelativePath { get; set; }
    public long Size { get; set; }
    public DateTime DateAdded { get; set; }
    public string? SceneName { get; set; }
    public string? ReleaseGroup { get; set; }
    public QualityModel? Quality { get; set; }
    public string? AudioFormat { get; set; }
    public int? Bitrate { get; set; }
    public int? SampleRate { get; set; }
    public int? Channels { get; set; }

    public override string ToString() => RelativePath ?? $"MusicFile {Id}";
}
