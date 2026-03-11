// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Manga;

public class MangaSeries : ModelBase
{
    public string Title { get; set; } = string.Empty;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public string? MangaDexId { get; set; }
    public int? AniListId { get; set; }
    public int? MyAnimeListId { get; set; }
    public string? Author { get; set; }
    public string? Artist { get; set; }
    public string? Status { get; set; }
    public int? Year { get; set; }
    public string? OriginalLanguage { get; set; }
    public string? ContentRating { get; set; }
    public string? Genres { get; set; }
    public string? Tags { get; set; }
    public string? CoverUrl { get; set; }
    public decimal? LastChapterNumber { get; set; }
    public int? LastVolumeNumber { get; set; }
    public int? ChapterCount { get; set; }
    public bool Monitored { get; set; } = true;
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; } = 1;
    public DateTime Added { get; set; }

    public override string ToString() => Title;
}
