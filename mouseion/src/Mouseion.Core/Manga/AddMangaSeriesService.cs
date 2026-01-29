// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Manga.AniList;
using Mouseion.Core.Manga.MangaDex;

namespace Mouseion.Core.Manga;

public interface IAddMangaSeriesService
{
    Task<MangaSeries> AddByMangaDexIdAsync(string mangaDexId, string? rootFolderPath = null, int qualityProfileId = 1, bool monitored = true, CancellationToken ct = default);
    Task<MangaSeries> AddByAniListIdAsync(int aniListId, string? rootFolderPath = null, int qualityProfileId = 1, bool monitored = true, CancellationToken ct = default);
    Task<List<MangaSeries>> SearchAsync(string query, CancellationToken ct = default);
}

public partial class AddMangaSeriesService : IAddMangaSeriesService
{
    private readonly IMangaSeriesRepository _seriesRepository;
    private readonly IMangaChapterRepository _chapterRepository;
    private readonly IMangaDexClient _mangaDexClient;
    private readonly IAniListClient _aniListClient;
    private readonly ILogger<AddMangaSeriesService> _logger;

    public AddMangaSeriesService(
        IMangaSeriesRepository seriesRepository,
        IMangaChapterRepository chapterRepository,
        IMangaDexClient mangaDexClient,
        IAniListClient aniListClient,
        ILogger<AddMangaSeriesService> logger)
    {
        _seriesRepository = seriesRepository;
        _chapterRepository = chapterRepository;
        _mangaDexClient = mangaDexClient;
        _aniListClient = aniListClient;
        _logger = logger;
    }

    public async Task<MangaSeries> AddByMangaDexIdAsync(
        string mangaDexId,
        string? rootFolderPath = null,
        int qualityProfileId = 1,
        bool monitored = true,
        CancellationToken ct = default)
    {
        var existing = await _seriesRepository.FindByMangaDexIdAsync(mangaDexId, ct).ConfigureAwait(false);
        if (existing != null)
        {
            LogMangaDexSeriesExists(mangaDexId);
            return existing;
        }

        var mangaDexManga = await _mangaDexClient.GetMangaAsync(mangaDexId, ct).ConfigureAwait(false);
        if (mangaDexManga == null)
        {
            throw new InvalidOperationException($"Could not find manga with MangaDex ID {mangaDexId}");
        }

        var series = MapFromMangaDex(mangaDexManga);
        series.RootFolderPath = rootFolderPath;
        series.QualityProfileId = qualityProfileId;
        series.Monitored = monitored;

        var coverRelationship = mangaDexManga.Relationships.FirstOrDefault(r => r.Type == "cover_art");
        if (coverRelationship != null)
        {
            series.CoverUrl = await _mangaDexClient.GetCoverUrlAsync(mangaDexId, coverRelationship.Id, ct).ConfigureAwait(false);
        }

        var insertedSeries = await _seriesRepository.InsertAsync(series, ct).ConfigureAwait(false);
        LogMangaSeriesAdded(insertedSeries.Title, insertedSeries.Id);

        var chapters = await _mangaDexClient.GetChaptersAsync(mangaDexId, "en", 100, 0, ct).ConfigureAwait(false);
        foreach (var mdChapter in chapters)
        {
            var chapter = MapChapterFromMangaDex(mdChapter, insertedSeries.Id);
            await _chapterRepository.InsertAsync(chapter, ct).ConfigureAwait(false);
        }

        LogChaptersAdded(chapters.Count, insertedSeries.Title);

        return insertedSeries;
    }

    public async Task<MangaSeries> AddByAniListIdAsync(
        int aniListId,
        string? rootFolderPath = null,
        int qualityProfileId = 1,
        bool monitored = true,
        CancellationToken ct = default)
    {
        var existing = await _seriesRepository.FindByAniListIdAsync(aniListId, ct).ConfigureAwait(false);
        if (existing != null)
        {
            LogAniListSeriesExists(aniListId);
            return existing;
        }

        var aniListMedia = await _aniListClient.GetMangaByIdAsync(aniListId, ct).ConfigureAwait(false);
        if (aniListMedia == null)
        {
            throw new InvalidOperationException($"Could not find manga with AniList ID {aniListId}");
        }

        var series = MapFromAniList(aniListMedia);
        series.RootFolderPath = rootFolderPath;
        series.QualityProfileId = qualityProfileId;
        series.Monitored = monitored;

        var insertedSeries = await _seriesRepository.InsertAsync(series, ct).ConfigureAwait(false);
        LogMangaSeriesAddedFromAniList(insertedSeries.Title, insertedSeries.Id);

        return insertedSeries;
    }

    public async Task<List<MangaSeries>> SearchAsync(string query, CancellationToken ct = default)
    {
        var results = await _mangaDexClient.SearchMangaAsync(query, 20, ct).ConfigureAwait(false);
        return results.Select(MapFromMangaDex).ToList();
    }

    private static MangaSeries MapFromMangaDex(MangaDexManga manga)
    {
        var title = manga.Attributes.Title.TryGetValue("en", out var enTitle)
            ? enTitle
            : manga.Attributes.Title.Values.FirstOrDefault() ?? "Unknown";

        var description = manga.Attributes.Description.TryGetValue("en", out var enDesc)
            ? enDesc
            : manga.Attributes.Description.Values.FirstOrDefault();

        var author = manga.Relationships
            .FirstOrDefault(r => r.Type == "author")?.Attributes?.GetProperty("name").GetString();

        var artist = manga.Relationships
            .FirstOrDefault(r => r.Type == "artist")?.Attributes?.GetProperty("name").GetString();

        var genres = manga.Attributes.Tags
            .Where(t => t.Attributes.Group == "genre")
            .Select(t => t.Attributes.Name.TryGetValue("en", out var name) ? name : t.Attributes.Name.Values.FirstOrDefault())
            .Where(g => g != null);

        var tags = manga.Attributes.Tags
            .Where(t => t.Attributes.Group != "genre")
            .Select(t => t.Attributes.Name.TryGetValue("en", out var name) ? name : t.Attributes.Name.Values.FirstOrDefault())
            .Where(t => t != null);

        decimal.TryParse(manga.Attributes.LastChapter, out var lastChapter);
        int.TryParse(manga.Attributes.LastVolume, out var lastVolume);

        return new MangaSeries
        {
            Title = title,
            SortTitle = title.ToLowerInvariant(),
            Description = description,
            MangaDexId = manga.Id,
            Author = author,
            Artist = artist,
            Status = MapMangaDexStatus(manga.Attributes.Status),
            Year = manga.Attributes.Year,
            OriginalLanguage = manga.Attributes.OriginalLanguage,
            ContentRating = manga.Attributes.ContentRating,
            Genres = string.Join(", ", genres),
            Tags = string.Join(", ", tags),
            LastChapterNumber = lastChapter > 0 ? lastChapter : null,
            LastVolumeNumber = lastVolume > 0 ? lastVolume : null,
            Added = DateTime.UtcNow
        };
    }

    private static MangaSeries MapFromAniList(AniListMedia media)
    {
        var title = media.Title?.GetPreferredTitle() ?? "Unknown";

        var author = media.Staff?.Edges?
            .FirstOrDefault(e => e.Role?.Contains("Story", StringComparison.OrdinalIgnoreCase) == true)?
            .Node?.Name?.Full;

        var artist = media.Staff?.Edges?
            .FirstOrDefault(e => e.Role?.Contains("Art", StringComparison.OrdinalIgnoreCase) == true)?
            .Node?.Name?.Full;

        return new MangaSeries
        {
            Title = title,
            SortTitle = title.ToLowerInvariant(),
            Description = media.Description,
            AniListId = media.Id,
            MyAnimeListId = media.IdMal,
            Author = author,
            Artist = artist,
            Status = MapAniListStatus(media.Status),
            Year = media.StartDate?.Year,
            Genres = media.Genres != null ? string.Join(", ", media.Genres) : null,
            Tags = media.Tags != null ? string.Join(", ", media.Tags.Take(10).Select(t => t.Name)) : null,
            CoverUrl = media.CoverImage?.Large ?? media.CoverImage?.Medium,
            ChapterCount = media.Chapters,
            LastVolumeNumber = media.Volumes,
            Added = DateTime.UtcNow
        };
    }

    private static MangaChapter MapChapterFromMangaDex(MangaDexChapter chapter, int seriesId)
    {
        decimal.TryParse(chapter.Attributes.Chapter, out var chapterNumber);
        int.TryParse(chapter.Attributes.Volume, out var volumeNumber);

        var scanlationGroup = chapter.Relationships
            .FirstOrDefault(r => r.Type == "scanlation_group")?.Attributes?.GetProperty("name").GetString();

        return new MangaChapter
        {
            MangaSeriesId = seriesId,
            Title = chapter.Attributes.Title,
            ChapterNumber = chapterNumber > 0 ? chapterNumber : null,
            VolumeNumber = volumeNumber > 0 ? volumeNumber : null,
            MangaDexChapterId = chapter.Id,
            ScanlationGroup = scanlationGroup,
            TranslatedLanguage = chapter.Attributes.TranslatedLanguage,
            PageCount = chapter.Attributes.Pages,
            ExternalUrl = chapter.Attributes.ExternalUrl,
            PublishDate = chapter.Attributes.PublishAt,
            Added = DateTime.UtcNow
        };
    }

    private static string MapMangaDexStatus(string status) => status switch
    {
        "ongoing" => "Ongoing",
        "completed" => "Completed",
        "hiatus" => "Hiatus",
        "cancelled" => "Cancelled",
        _ => "Unknown"
    };

    private static string MapAniListStatus(string? status) => status switch
    {
        "FINISHED" => "Completed",
        "RELEASING" => "Ongoing",
        "NOT_YET_RELEASED" => "Not Yet Released",
        "CANCELLED" => "Cancelled",
        "HIATUS" => "Hiatus",
        _ => "Unknown"
    };

    [LoggerMessage(Level = LogLevel.Information, Message = "Manga series with MangaDex ID {MangaDexId} already exists")]
    private partial void LogMangaDexSeriesExists(string mangaDexId);

    [LoggerMessage(Level = LogLevel.Information, Message = "Added manga series {Title} (ID: {Id})")]
    private partial void LogMangaSeriesAdded(string title, int id);

    [LoggerMessage(Level = LogLevel.Information, Message = "Added {Count} chapters for manga {Title}")]
    private partial void LogChaptersAdded(int count, string title);

    [LoggerMessage(Level = LogLevel.Information, Message = "Manga series with AniList ID {AniListId} already exists")]
    private partial void LogAniListSeriesExists(int aniListId);

    [LoggerMessage(Level = LogLevel.Information, Message = "Added manga series {Title} from AniList (ID: {Id})")]
    private partial void LogMangaSeriesAddedFromAniList(string title, int id);
}
