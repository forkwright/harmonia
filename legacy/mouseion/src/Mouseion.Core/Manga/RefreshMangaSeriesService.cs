// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Manga.MangaDex;

namespace Mouseion.Core.Manga;

public interface IRefreshMangaSeriesService
{
    Task<int> RefreshSeriesAsync(int seriesId, CancellationToken ct = default);
    Task<int> RefreshAllMonitoredAsync(CancellationToken ct = default);
}

public partial class RefreshMangaSeriesService : IRefreshMangaSeriesService
{
    private readonly IMangaSeriesRepository _seriesRepository;
    private readonly IMangaChapterRepository _chapterRepository;
    private readonly IMangaDexClient _mangaDexClient;
    private readonly ILogger<RefreshMangaSeriesService> _logger;

    public RefreshMangaSeriesService(
        IMangaSeriesRepository seriesRepository,
        IMangaChapterRepository chapterRepository,
        IMangaDexClient mangaDexClient,
        ILogger<RefreshMangaSeriesService> logger)
    {
        _seriesRepository = seriesRepository;
        _chapterRepository = chapterRepository;
        _mangaDexClient = mangaDexClient;
        _logger = logger;
    }

    public async Task<int> RefreshSeriesAsync(int seriesId, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(seriesId, ct).ConfigureAwait(false);
        if (series == null)
        {
            LogSeriesNotFound(seriesId);
            return 0;
        }

        if (string.IsNullOrEmpty(series.MangaDexId))
        {
            LogSeriesNoMangaDexId(seriesId);
            return 0;
        }

        var newChaptersAdded = 0;
        var chapters = await _mangaDexClient.GetChaptersAsync(series.MangaDexId, "en", 100, 0, ct).ConfigureAwait(false);

        foreach (var mdChapter in chapters)
        {
            var existing = await _chapterRepository.FindByMangaDexChapterIdAsync(mdChapter.Id, ct).ConfigureAwait(false);
            if (existing != null)
            {
                continue;
            }

            var chapter = MapChapterFromMangaDex(mdChapter, seriesId);
            await _chapterRepository.InsertAsync(chapter, ct).ConfigureAwait(false);
            newChaptersAdded++;
        }

        if (newChaptersAdded > 0)
        {
            LogNewChaptersAdded(newChaptersAdded, series.Title);

            var allChapters = await _chapterRepository.GetBySeriesIdAsync(seriesId, ct).ConfigureAwait(false);
            series.ChapterCount = allChapters.Count;
            series.LastChapterNumber = allChapters.Max(c => c.ChapterNumber);
            series.LastVolumeNumber = allChapters.Max(c => c.VolumeNumber);
            await _seriesRepository.UpdateAsync(series, ct).ConfigureAwait(false);
        }

        return newChaptersAdded;
    }

    public async Task<int> RefreshAllMonitoredAsync(CancellationToken ct = default)
    {
        var monitoredSeries = await _seriesRepository.GetMonitoredAsync(ct).ConfigureAwait(false);
        var totalNewChapters = 0;

        foreach (var series in monitoredSeries)
        {
            try
            {
                var newChapters = await RefreshSeriesAsync(series.Id, ct).ConfigureAwait(false);
                totalNewChapters += newChapters;
            }
            catch (Exception ex)
            {
                LogRefreshFailed(ex, series.Title);
            }
        }

        LogRefreshCompleted(monitoredSeries.Count, totalNewChapters);

        return totalNewChapters;
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

    [LoggerMessage(Level = LogLevel.Warning, Message = "Manga series {SeriesId} not found")]
    private partial void LogSeriesNotFound(int seriesId);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Manga series {SeriesId} has no MangaDex ID, cannot refresh")]
    private partial void LogSeriesNoMangaDexId(int seriesId);

    [LoggerMessage(Level = LogLevel.Information, Message = "Added {Count} new chapters to manga {Title}")]
    private partial void LogNewChaptersAdded(int count, string title);

    [LoggerMessage(Level = LogLevel.Error, Message = "Failed to refresh manga series {Title}")]
    private partial void LogRefreshFailed(Exception ex, string title);

    [LoggerMessage(Level = LogLevel.Information, Message = "Refreshed {Count} monitored manga series, found {NewChapters} new chapters")]
    private partial void LogRefreshCompleted(int count, int newChapters);
}
