// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Manga;

public class MangaChapter : ModelBase
{
    public int MangaSeriesId { get; set; }
    public string? Title { get; set; }
    public decimal? ChapterNumber { get; set; }
    public int? VolumeNumber { get; set; }
    public string? MangaDexChapterId { get; set; }
    public string? ScanlationGroup { get; set; }
    public string? TranslatedLanguage { get; set; }
    public int? PageCount { get; set; }
    public string? ExternalUrl { get; set; }
    public DateTime? PublishDate { get; set; }
    public bool IsRead { get; set; }
    public bool IsDownloaded { get; set; }
    public string? FilePath { get; set; }
    public DateTime Added { get; set; }

    public override string ToString() => Title ?? $"Chapter {ChapterNumber}";
}
