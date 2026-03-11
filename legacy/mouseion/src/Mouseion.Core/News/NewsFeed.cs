// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.News;

public class NewsFeed : ModelBase
{
    public string Title { get; set; } = string.Empty;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public string FeedUrl { get; set; } = string.Empty;
    public string? SiteUrl { get; set; }
    public string? FaviconUrl { get; set; }
    public string? Category { get; set; }
    public string? Language { get; set; }
    public DateTime? LastFetchTime { get; set; }
    public DateTime? LastItemDate { get; set; }
    public int? ItemCount { get; set; }
    public bool Monitored { get; set; } = true;
    public int RefreshInterval { get; set; } = 60;
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; } = 1;
    public string? Tags { get; set; }
    public DateTime Added { get; set; }

    public override string ToString() => Title;
}
