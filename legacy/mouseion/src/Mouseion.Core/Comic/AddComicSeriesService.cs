// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Comic.ComicVine;

namespace Mouseion.Core.Comic;

public interface IAddComicSeriesService
{
    Task<ComicSeries?> AddByComicVineIdAsync(int comicVineId, string? rootFolderPath = null, int qualityProfileId = 1, CancellationToken ct = default);
    Task<List<ComicVineVolume>> SearchAsync(string query, int limit = 10, CancellationToken ct = default);
}

public partial class AddComicSeriesService : IAddComicSeriesService
{
    private readonly IComicSeriesRepository _seriesRepository;
    private readonly IComicVineClient _comicVineClient;
    private readonly ILogger<AddComicSeriesService> _logger;

    public AddComicSeriesService(
        IComicSeriesRepository seriesRepository,
        IComicVineClient comicVineClient,
        ILogger<AddComicSeriesService> logger)
    {
        _seriesRepository = seriesRepository;
        _comicVineClient = comicVineClient;
        _logger = logger;
    }

    public async Task<ComicSeries?> AddByComicVineIdAsync(int comicVineId, string? rootFolderPath = null, int qualityProfileId = 1, CancellationToken ct = default)
    {
        var existing = await _seriesRepository.FindByComicVineIdAsync(comicVineId, ct).ConfigureAwait(false);
        if (existing != null)
        {
            LogSeriesAlreadyExists(comicVineId, existing.Title);
            return existing;
        }

        var volume = await _comicVineClient.GetVolumeAsync(comicVineId, ct).ConfigureAwait(false);
        if (volume == null)
        {
            LogVolumeNotFound(comicVineId);
            return null;
        }

        var series = new ComicSeries
        {
            Title = volume.Name ?? "Unknown",
            SortTitle = volume.Name?.ToLowerInvariant(),
            Description = StripHtml(volume.Description),
            ComicVineId = volume.Id,
            Publisher = volume.Publisher?.Name,
            StartYear = int.TryParse(volume.StartYear, out var year) ? year : null,
            IssueCount = volume.CountOfIssues,
            CoverUrl = volume.Image?.OriginalUrl ?? volume.Image?.SuperUrl,
            SiteUrl = volume.SiteDetailUrl,
            Monitored = true,
            RootFolderPath = rootFolderPath,
            QualityProfileId = qualityProfileId,
            Added = DateTime.UtcNow
        };

        var inserted = await _seriesRepository.InsertAsync(series, ct).ConfigureAwait(false);
        LogSeriesAdded(inserted.Title, comicVineId);

        return inserted;
    }

    public async Task<List<ComicVineVolume>> SearchAsync(string query, int limit = 10, CancellationToken ct = default)
    {
        return await _comicVineClient.SearchVolumesAsync(query, limit, ct).ConfigureAwait(false);
    }

    private static string? StripHtml(string? html)
    {
        if (string.IsNullOrEmpty(html))
        {
            return null;
        }

        return System.Text.RegularExpressions.Regex.Replace(html, "<[^>]*>", string.Empty).Trim();
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Comic series with ComicVine ID {ComicVineId} already exists: {Title}")]
    private partial void LogSeriesAlreadyExists(int comicVineId, string title);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Could not find volume with ComicVine ID {ComicVineId}")]
    private partial void LogVolumeNotFound(int comicVineId);

    [LoggerMessage(Level = LogLevel.Information, Message = "Added comic series: {Title} (ComicVine ID: {ComicVineId})")]
    private partial void LogSeriesAdded(string title, int comicVineId);
}
