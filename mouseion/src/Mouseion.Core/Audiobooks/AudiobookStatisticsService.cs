// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;

namespace Mouseion.Core.Audiobooks;

public interface IAudiobookStatisticsService
{
    Task<AudiobookStatistics> GetStatisticsAsync(CancellationToken ct = default);
    Task<AudiobookStatistics> GetAuthorStatisticsAsync(int authorId, CancellationToken ct = default);
    Task<AudiobookStatistics> GetSeriesStatisticsAsync(int seriesId, CancellationToken ct = default);
    Task<AudiobookStatistics> GetNarratorStatisticsAsync(string narrator, CancellationToken ct = default);

    AudiobookStatistics GetStatistics();
    AudiobookStatistics GetAuthorStatistics(int authorId);
    AudiobookStatistics GetSeriesStatistics(int seriesId);
    AudiobookStatistics GetNarratorStatistics(string narrator);
}

public class AudiobookStatisticsService : IAudiobookStatisticsService
{
    private readonly IAudiobookRepository _audiobookRepository;
    private readonly ILogger<AudiobookStatisticsService> _logger;

    public AudiobookStatisticsService(
        IAudiobookRepository audiobookRepository,
        ILogger<AudiobookStatisticsService> logger)
    {
        _audiobookRepository = audiobookRepository;
        _logger = logger;
    }

    public async Task<AudiobookStatistics> GetStatisticsAsync(CancellationToken ct = default)
    {
        var allAudiobooksEnum = await _audiobookRepository.AllAsync(ct).ConfigureAwait(false);
        var allAudiobooks = allAudiobooksEnum.ToList();

        var stats = new AudiobookStatistics
        {
            TotalCount = allAudiobooks.Count,
            MonitoredCount = allAudiobooks.Count(a => a.Monitored),
            UnmonitoredCount = allAudiobooks.Count(a => !a.Monitored),
            TotalDurationMinutes = allAudiobooks.Sum(a => a.Metadata.DurationMinutes ?? 0),
            ByYear = allAudiobooks.GroupBy(a => a.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .Take(10)
                .ToDictionary(g => g.Key, g => g.Count()),
            ByNarrator = allAudiobooks
                .Where(a => !string.IsNullOrWhiteSpace(a.Metadata.Narrator))
                .GroupBy(a => a.Metadata.Narrator!)
                .OrderByDescending(g => g.Count())
                .Take(10)
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Audiobook statistics: {TotalCount} total, {MonitoredCount} monitored, {TotalHours} hours",
            stats.TotalCount, stats.MonitoredCount, stats.TotalDurationMinutes / 60);

        return stats;
    }

    public AudiobookStatistics GetStatistics()
    {
        var allAudiobooks = _audiobookRepository.All().ToList();

        var stats = new AudiobookStatistics
        {
            TotalCount = allAudiobooks.Count,
            MonitoredCount = allAudiobooks.Count(a => a.Monitored),
            UnmonitoredCount = allAudiobooks.Count(a => !a.Monitored),
            TotalDurationMinutes = allAudiobooks.Sum(a => a.Metadata.DurationMinutes ?? 0),
            ByYear = allAudiobooks.GroupBy(a => a.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .Take(10)
                .ToDictionary(g => g.Key, g => g.Count()),
            ByNarrator = allAudiobooks
                .Where(a => !string.IsNullOrWhiteSpace(a.Metadata.Narrator))
                .GroupBy(a => a.Metadata.Narrator!)
                .OrderByDescending(g => g.Count())
                .Take(10)
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Audiobook statistics: {TotalCount} total, {MonitoredCount} monitored, {TotalHours} hours",
            stats.TotalCount, stats.MonitoredCount, stats.TotalDurationMinutes / 60);

        return stats;
    }

    public async Task<AudiobookStatistics> GetAuthorStatisticsAsync(int authorId, CancellationToken ct = default)
    {
        var authorAudiobooks = await _audiobookRepository.GetByAuthorIdAsync(authorId, ct).ConfigureAwait(false);

        var stats = new AudiobookStatistics
        {
            TotalCount = authorAudiobooks.Count,
            MonitoredCount = authorAudiobooks.Count(a => a.Monitored),
            UnmonitoredCount = authorAudiobooks.Count(a => !a.Monitored),
            TotalDurationMinutes = authorAudiobooks.Sum(a => a.Metadata.DurationMinutes ?? 0),
            ByYear = authorAudiobooks.GroupBy(a => a.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .ToDictionary(g => g.Key, g => g.Count()),
            ByNarrator = authorAudiobooks
                .Where(a => !string.IsNullOrWhiteSpace(a.Metadata.Narrator))
                .GroupBy(a => a.Metadata.Narrator!)
                .OrderByDescending(g => g.Count())
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Author {AuthorId} audiobook statistics: {TotalCount} total",
            authorId, stats.TotalCount);

        return stats;
    }

    public AudiobookStatistics GetAuthorStatistics(int authorId)
    {
        var authorAudiobooks = _audiobookRepository.GetByAuthorId(authorId);

        var stats = new AudiobookStatistics
        {
            TotalCount = authorAudiobooks.Count,
            MonitoredCount = authorAudiobooks.Count(a => a.Monitored),
            UnmonitoredCount = authorAudiobooks.Count(a => !a.Monitored),
            TotalDurationMinutes = authorAudiobooks.Sum(a => a.Metadata.DurationMinutes ?? 0),
            ByYear = authorAudiobooks.GroupBy(a => a.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .ToDictionary(g => g.Key, g => g.Count()),
            ByNarrator = authorAudiobooks
                .Where(a => !string.IsNullOrWhiteSpace(a.Metadata.Narrator))
                .GroupBy(a => a.Metadata.Narrator!)
                .OrderByDescending(g => g.Count())
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Author {AuthorId} audiobook statistics: {TotalCount} total",
            authorId, stats.TotalCount);

        return stats;
    }

    public async Task<AudiobookStatistics> GetSeriesStatisticsAsync(int seriesId, CancellationToken ct = default)
    {
        var seriesAudiobooks = await _audiobookRepository.GetBySeriesIdAsync(seriesId, ct).ConfigureAwait(false);

        var stats = new AudiobookStatistics
        {
            TotalCount = seriesAudiobooks.Count,
            MonitoredCount = seriesAudiobooks.Count(a => a.Monitored),
            UnmonitoredCount = seriesAudiobooks.Count(a => !a.Monitored),
            TotalDurationMinutes = seriesAudiobooks.Sum(a => a.Metadata.DurationMinutes ?? 0),
            ByYear = seriesAudiobooks.GroupBy(a => a.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .ToDictionary(g => g.Key, g => g.Count()),
            ByNarrator = seriesAudiobooks
                .Where(a => !string.IsNullOrWhiteSpace(a.Metadata.Narrator))
                .GroupBy(a => a.Metadata.Narrator!)
                .OrderByDescending(g => g.Count())
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Series {SeriesId} audiobook statistics: {TotalCount} total",
            seriesId, stats.TotalCount);

        return stats;
    }

    public AudiobookStatistics GetSeriesStatistics(int seriesId)
    {
        var seriesAudiobooks = _audiobookRepository.GetBySeriesId(seriesId);

        var stats = new AudiobookStatistics
        {
            TotalCount = seriesAudiobooks.Count,
            MonitoredCount = seriesAudiobooks.Count(a => a.Monitored),
            UnmonitoredCount = seriesAudiobooks.Count(a => !a.Monitored),
            TotalDurationMinutes = seriesAudiobooks.Sum(a => a.Metadata.DurationMinutes ?? 0),
            ByYear = seriesAudiobooks.GroupBy(a => a.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .ToDictionary(g => g.Key, g => g.Count()),
            ByNarrator = seriesAudiobooks
                .Where(a => !string.IsNullOrWhiteSpace(a.Metadata.Narrator))
                .GroupBy(a => a.Metadata.Narrator!)
                .OrderByDescending(g => g.Count())
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Series {SeriesId} audiobook statistics: {TotalCount} total",
            seriesId, stats.TotalCount);

        return stats;
    }

    public async Task<AudiobookStatistics> GetNarratorStatisticsAsync(string narrator, CancellationToken ct = default)
    {
        var allAudiobooksEnum = await _audiobookRepository.AllAsync(ct).ConfigureAwait(false);
        var allAudiobooks = allAudiobooksEnum
            .Where(a => a.Metadata.Narrator?.Equals(narrator, StringComparison.OrdinalIgnoreCase) == true)
            .ToList();

        var stats = new AudiobookStatistics
        {
            TotalCount = allAudiobooks.Count,
            MonitoredCount = allAudiobooks.Count(a => a.Monitored),
            UnmonitoredCount = allAudiobooks.Count(a => !a.Monitored),
            TotalDurationMinutes = allAudiobooks.Sum(a => a.Metadata.DurationMinutes ?? 0),
            ByYear = allAudiobooks.GroupBy(a => a.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Narrator '{Narrator}' audiobook statistics: {TotalCount} total",
            narrator, stats.TotalCount);

        return stats;
    }

    public AudiobookStatistics GetNarratorStatistics(string narrator)
    {
        var allAudiobooks = _audiobookRepository.All()
            .Where(a => a.Metadata.Narrator?.Equals(narrator, StringComparison.OrdinalIgnoreCase) == true)
            .ToList();

        var stats = new AudiobookStatistics
        {
            TotalCount = allAudiobooks.Count,
            MonitoredCount = allAudiobooks.Count(a => a.Monitored),
            UnmonitoredCount = allAudiobooks.Count(a => !a.Monitored),
            TotalDurationMinutes = allAudiobooks.Sum(a => a.Metadata.DurationMinutes ?? 0),
            ByYear = allAudiobooks.GroupBy(a => a.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Narrator '{Narrator}' audiobook statistics: {TotalCount} total",
            narrator, stats.TotalCount);

        return stats;
    }
}

public class AudiobookStatistics
{
    public int TotalCount { get; set; }
    public int MonitoredCount { get; set; }
    public int UnmonitoredCount { get; set; }
    public int TotalDurationMinutes { get; set; }
    public Dictionary<int, int> ByYear { get; set; } = new();
    public Dictionary<string, int> ByNarrator { get; set; } = new();
}
