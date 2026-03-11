// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Podcasts;

public class PodcastFile : ModelBase
{
    public int PodcastEpisodeId { get; set; }
    public int? PodcastShowId { get; set; }
    public string RelativePath { get; set; } = string.Empty;
    public string? Path { get; set; }
    public long Size { get; set; }
    public DateTime DateAdded { get; set; }
    public string? SceneName { get; set; }
    public string? ReleaseGroup { get; set; }
    public string? Quality { get; set; }
    public string? AudioFormat { get; set; }
    public int? Bitrate { get; set; }
    public int? SampleRate { get; set; }
    public int? Channels { get; set; }
    public int? Duration { get; set; }

    public override string ToString() => RelativePath ?? Path ?? string.Empty;
}
