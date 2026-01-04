// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Podcasts;

public class PodcastShow : ModelBase
{
    public string Title { get; set; } = string.Empty;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public string? ForeignPodcastId { get; set; }
    public string? ItunesId { get; set; }
    public string? Author { get; set; }
    public string FeedUrl { get; set; } = string.Empty;
    public string? ImageUrl { get; set; }
    public string? Categories { get; set; }
    public string? Language { get; set; }
    public string? Website { get; set; }
    public int? EpisodeCount { get; set; }
    public DateTime? LatestEpisodeDate { get; set; }
    public bool Monitored { get; set; }
    public bool MonitorNewEpisodes { get; set; } = true;
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; } = 1;
    public string? Tags { get; set; }
    public DateTime Added { get; set; }
    public DateTime? LastSearchTime { get; set; }

    public override string ToString() => Title;
}
