// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Comic;

public class ComicSeries : ModelBase
{
    public string Title { get; set; } = string.Empty;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public int? ComicVineId { get; set; }
    public string? Publisher { get; set; }
    public string? Imprint { get; set; }
    public int? StartYear { get; set; }
    public int? EndYear { get; set; }
    public string? Status { get; set; }
    public int? IssueCount { get; set; }
    public int? VolumeNumber { get; set; }
    public string? Genres { get; set; }
    public string? Characters { get; set; }
    public string? CoverUrl { get; set; }
    public string? SiteUrl { get; set; }
    public bool Monitored { get; set; } = true;
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; } = 1;
    public DateTime Added { get; set; }

    public override string ToString() => Title;
}
