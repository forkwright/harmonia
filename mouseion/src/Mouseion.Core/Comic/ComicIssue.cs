// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Comic;

public class ComicIssue : ModelBase
{
    public int ComicSeriesId { get; set; }
    public string? Title { get; set; }
    public string? IssueNumber { get; set; }
    public int? ComicVineIssueId { get; set; }
    public string? StoryArc { get; set; }
    public string? Writer { get; set; }
    public string? Penciler { get; set; }
    public string? Inker { get; set; }
    public string? Colorist { get; set; }
    public string? CoverArtist { get; set; }
    public DateTime? CoverDate { get; set; }
    public DateTime? StoreDate { get; set; }
    public int? PageCount { get; set; }
    public string? CoverUrl { get; set; }
    public string? SiteUrl { get; set; }
    public string? Description { get; set; }
    public bool IsRead { get; set; }
    public bool IsDownloaded { get; set; }
    public string? FilePath { get; set; }
    public string? FileFormat { get; set; }
    public DateTime Added { get; set; }

    public override string ToString() => Title ?? $"Issue #{IssueNumber}";
}
