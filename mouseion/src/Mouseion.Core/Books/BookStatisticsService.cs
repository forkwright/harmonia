// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Books;

public interface IBookStatisticsService
{
    Task<BookStatistics> GetStatisticsAsync(CancellationToken ct = default);
    Task<BookStatistics> GetAuthorStatisticsAsync(int authorId, CancellationToken ct = default);
    Task<BookStatistics> GetSeriesStatisticsAsync(int seriesId, CancellationToken ct = default);

    BookStatistics GetStatistics();
    BookStatistics GetAuthorStatistics(int authorId);
    BookStatistics GetSeriesStatistics(int seriesId);
}

public class BookStatisticsService : IBookStatisticsService
{
    private readonly IBookRepository _bookRepository;
    private readonly ILogger<BookStatisticsService> _logger;

    public BookStatisticsService(IBookRepository bookRepository, ILogger<BookStatisticsService> logger)
    {
        _bookRepository = bookRepository;
        _logger = logger;
    }

    public async Task<BookStatistics> GetStatisticsAsync(CancellationToken ct = default)
    {
        var allBooksEnum = await _bookRepository.AllAsync(ct).ConfigureAwait(false);
        var allBooks = allBooksEnum.ToList();

        var stats = new BookStatistics
        {
            TotalCount = allBooks.Count,
            MonitoredCount = allBooks.Count(b => b.Monitored),
            UnmonitoredCount = allBooks.Count(b => !b.Monitored),
            ByYear = allBooks.GroupBy(b => b.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .Take(10)
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Book statistics: {TotalCount} total, {MonitoredCount} monitored",
            stats.TotalCount, stats.MonitoredCount);

        return stats;
    }

    public BookStatistics GetStatistics()
    {
        var allBooks = _bookRepository.All().ToList();

        var stats = new BookStatistics
        {
            TotalCount = allBooks.Count,
            MonitoredCount = allBooks.Count(b => b.Monitored),
            UnmonitoredCount = allBooks.Count(b => !b.Monitored),
            ByYear = allBooks.GroupBy(b => b.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .Take(10)
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Book statistics: {TotalCount} total, {MonitoredCount} monitored",
            stats.TotalCount, stats.MonitoredCount);

        return stats;
    }

    public async Task<BookStatistics> GetAuthorStatisticsAsync(int authorId, CancellationToken ct = default)
    {
        var authorBooks = await _bookRepository.GetByAuthorIdAsync(authorId, ct).ConfigureAwait(false);

        var stats = new BookStatistics
        {
            TotalCount = authorBooks.Count,
            MonitoredCount = authorBooks.Count(b => b.Monitored),
            UnmonitoredCount = authorBooks.Count(b => !b.Monitored),
            ByYear = authorBooks.GroupBy(b => b.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Author {AuthorId} book statistics: {TotalCount} total",
            authorId, stats.TotalCount);

        return stats;
    }

    public BookStatistics GetAuthorStatistics(int authorId)
    {
        var authorBooks = _bookRepository.GetByAuthorId(authorId);

        var stats = new BookStatistics
        {
            TotalCount = authorBooks.Count,
            MonitoredCount = authorBooks.Count(b => b.Monitored),
            UnmonitoredCount = authorBooks.Count(b => !b.Monitored),
            ByYear = authorBooks.GroupBy(b => b.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Author {AuthorId} book statistics: {TotalCount} total",
            authorId, stats.TotalCount);

        return stats;
    }

    public async Task<BookStatistics> GetSeriesStatisticsAsync(int seriesId, CancellationToken ct = default)
    {
        var seriesBooks = await _bookRepository.GetBySeriesIdAsync(seriesId, ct).ConfigureAwait(false);

        var stats = new BookStatistics
        {
            TotalCount = seriesBooks.Count,
            MonitoredCount = seriesBooks.Count(b => b.Monitored),
            UnmonitoredCount = seriesBooks.Count(b => !b.Monitored),
            ByYear = seriesBooks.GroupBy(b => b.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Series {SeriesId} book statistics: {TotalCount} total",
            seriesId, stats.TotalCount);

        return stats;
    }

    public BookStatistics GetSeriesStatistics(int seriesId)
    {
        var seriesBooks = _bookRepository.GetBySeriesId(seriesId);

        var stats = new BookStatistics
        {
            TotalCount = seriesBooks.Count,
            MonitoredCount = seriesBooks.Count(b => b.Monitored),
            UnmonitoredCount = seriesBooks.Count(b => !b.Monitored),
            ByYear = seriesBooks.GroupBy(b => b.Year)
                .Where(g => g.Key > 0)
                .OrderByDescending(g => g.Key)
                .ToDictionary(g => g.Key, g => g.Count())
        };

        _logger.LogDebug("Series {SeriesId} book statistics: {TotalCount} total",
            seriesId, stats.TotalCount);

        return stats;
    }
}

public class BookStatistics
{
    public int TotalCount { get; set; }
    public int MonitoredCount { get; set; }
    public int UnmonitoredCount { get; set; }
    public Dictionary<int, int> ByYear { get; set; } = new();
}
