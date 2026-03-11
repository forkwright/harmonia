// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Comic.ComicVine;

namespace Mouseion.Core.Comic;

public interface IRefreshComicSeriesService
{
    Task<int> RefreshSeriesAsync(int seriesId, CancellationToken ct = default);
    Task<int> RefreshAllMonitoredAsync(CancellationToken ct = default);
}

public partial class RefreshComicSeriesService : IRefreshComicSeriesService
{
    private readonly IComicSeriesRepository _seriesRepository;
    private readonly IComicIssueRepository _issueRepository;
    private readonly IComicVineClient _comicVineClient;
    private readonly ILogger<RefreshComicSeriesService> _logger;

    public RefreshComicSeriesService(
        IComicSeriesRepository seriesRepository,
        IComicIssueRepository issueRepository,
        IComicVineClient comicVineClient,
        ILogger<RefreshComicSeriesService> logger)
    {
        _seriesRepository = seriesRepository;
        _issueRepository = issueRepository;
        _comicVineClient = comicVineClient;
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

        if (!series.ComicVineId.HasValue)
        {
            LogSeriesNoComicVineId(series.Title);
            return 0;
        }

        var issues = await _comicVineClient.GetIssuesForVolumeAsync(series.ComicVineId.Value, ct: ct).ConfigureAwait(false);
        var newIssueCount = 0;

        foreach (var cvIssue in issues)
        {
            var existing = await _issueRepository.FindByComicVineIssueIdAsync(cvIssue.Id, ct).ConfigureAwait(false);
            if (existing != null)
            {
                continue;
            }

            var issue = new ComicIssue
            {
                ComicSeriesId = seriesId,
                Title = cvIssue.Name,
                IssueNumber = cvIssue.IssueNumber,
                ComicVineIssueId = cvIssue.Id,
                Description = StripHtml(cvIssue.Description),
                CoverDate = ParseDate(cvIssue.CoverDate),
                StoreDate = ParseDate(cvIssue.StoreDate),
                CoverUrl = cvIssue.Image?.OriginalUrl ?? cvIssue.Image?.SuperUrl,
                SiteUrl = cvIssue.SiteDetailUrl,
                Writer = GetCredit(cvIssue.PersonCredits, "writer"),
                Penciler = GetCredit(cvIssue.PersonCredits, "penciler"),
                Inker = GetCredit(cvIssue.PersonCredits, "inker"),
                Colorist = GetCredit(cvIssue.PersonCredits, "colorist"),
                CoverArtist = GetCredit(cvIssue.PersonCredits, "cover"),
                IsRead = false,
                IsDownloaded = false,
                Added = DateTime.UtcNow
            };

            await _issueRepository.InsertAsync(issue, ct).ConfigureAwait(false);
            newIssueCount++;
        }

        if (newIssueCount > 0)
        {
            LogIssuesAdded(newIssueCount, series.Title);
        }

        return newIssueCount;
    }

    public async Task<int> RefreshAllMonitoredAsync(CancellationToken ct = default)
    {
        var monitoredSeries = await _seriesRepository.GetMonitoredAsync(ct).ConfigureAwait(false);
        var totalNewIssues = 0;

        foreach (var series in monitoredSeries)
        {
            var newIssues = await RefreshSeriesAsync(series.Id, ct).ConfigureAwait(false);
            totalNewIssues += newIssues;
        }

        return totalNewIssues;
    }

    private static string? StripHtml(string? html)
    {
        if (string.IsNullOrEmpty(html))
        {
            return null;
        }

        return System.Text.RegularExpressions.Regex.Replace(html, "<[^>]*>", string.Empty).Trim();
    }

    private static DateTime? ParseDate(string? dateStr)
    {
        if (string.IsNullOrEmpty(dateStr))
        {
            return null;
        }

        if (DateTime.TryParse(dateStr, out var date))
        {
            return date;
        }

        return null;
    }

    private static string? GetCredit(List<ComicVinePersonCredit>? credits, string role)
    {
        if (credits == null || credits.Count == 0)
        {
            return null;
        }

        var matching = credits
            .Where(c => c.Role?.Contains(role, StringComparison.OrdinalIgnoreCase) == true)
            .Select(c => c.Name)
            .Where(n => !string.IsNullOrEmpty(n))
            .ToList();

        return matching.Count > 0 ? string.Join(", ", matching) : null;
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Comic series {SeriesId} not found")]
    private partial void LogSeriesNotFound(int seriesId);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Comic series {Title} has no ComicVine ID")]
    private partial void LogSeriesNoComicVineId(string title);

    [LoggerMessage(Level = LogLevel.Information, Message = "Added {Count} new issues to {Title}")]
    private partial void LogIssuesAdded(int count, string title);
}
