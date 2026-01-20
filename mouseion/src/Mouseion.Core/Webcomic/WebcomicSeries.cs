// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Webcomic;

public class WebcomicSeries : ModelBase
{
    public string Title { get; set; } = string.Empty;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public string? WebtoonId { get; set; }
    public string? TapasId { get; set; }
    public string? Author { get; set; }
    public string? Artist { get; set; }
    public string? Status { get; set; }
    public string? Platform { get; set; }
    public string? UpdateSchedule { get; set; }
    public string? Genres { get; set; }
    public string? Tags { get; set; }
    public string? CoverUrl { get; set; }
    public string? ThumbnailUrl { get; set; }
    public string? SiteUrl { get; set; }
    public int? LastEpisodeNumber { get; set; }
    public int? EpisodeCount { get; set; }
    public bool Monitored { get; set; } = true;
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; } = 1;
    public DateTime Added { get; set; }

    public override string ToString() => Title;
}
