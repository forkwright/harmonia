// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Analytics;

public interface IAnalyticsService
{
    Task<ConsumptionStats> GetConsumptionStatsAsync(string userId, DateRange period, CancellationToken ct = default);
    Task<TasteProfile> GetTasteProfileAsync(string userId, CancellationToken ct = default);
}

public class AnalyticsService : IAnalyticsService
{
    private readonly IAnalyticsRepository _repository;
    private readonly ILogger<AnalyticsService> _logger;

    public AnalyticsService(IAnalyticsRepository repository, ILogger<AnalyticsService> logger)
    {
        _repository = repository;
        _logger = logger;
    }

    public async Task<ConsumptionStats> GetConsumptionStatsAsync(
        string userId, DateRange period, CancellationToken ct = default)
    {
        var stats = new ConsumptionStats
        {
            UserId = userId,
            Period = period
        };

        // Gather data in parallel
        var completedTask = _repository.GetCompletedCountsByTypeAsync(userId, period, ct);
        var inProgressTask = _repository.GetInProgressCountsByTypeAsync(userId, ct);
        var totalTask = _repository.GetTotalCountsByTypeAsync(ct);
        var sessionsTask = _repository.GetSessionAggregatesAsync(userId, period, ct);
        var dailyTask = _repository.GetDailySessionCountsAsync(userId, period, ct);
        var hourlyTask = _repository.GetHourlyActivityAsync(userId, period, ct);
        var dowTask = _repository.GetDayOfWeekActivityAsync(userId, period, ct);

        await Task.WhenAll(completedTask, inProgressTask, totalTask, sessionsTask,
            dailyTask, hourlyTask, dowTask).ConfigureAwait(false);

        var completed = await completedTask;
        var inProgress = await inProgressTask;
        var totals = await totalTask;
        var sessions = (await sessionsTask).FirstOrDefault() ?? new SessionAggregate();
        var dailyCounts = await dailyTask;
        var hourly = await hourlyTask;
        var dow = await dowTask;

        // Build per-media-type stats
        var allTypes = completed.Select(c => c.MediaType)
            .Union(inProgress.Select(i => i.MediaType))
            .Union(totals.Select(t => t.MediaType))
            .Distinct();

        foreach (var mediaType in allTypes)
        {
            var typeStats = new MediaTypeStats
            {
                MediaType = mediaType,
                ItemsCompleted = completed.FirstOrDefault(c => c.MediaType == mediaType)?.Count ?? 0,
                ItemsInProgress = inProgress.FirstOrDefault(i => i.MediaType == mediaType)?.Count ?? 0,
                ItemsTotal = totals.FirstOrDefault(t => t.MediaType == mediaType)?.Count ?? 0
            };

            // Set media-specific counters
            switch (mediaType)
            {
                case MediaType.Movie:
                case MediaType.TV:
                    typeStats.EpisodesWatched = typeStats.ItemsCompleted;
                    break;
                case MediaType.Podcast:
                    typeStats.EpisodesListened = typeStats.ItemsCompleted;
                    break;
            }

            stats.ByMediaType.Add(typeStats);
        }

        // Overall stats
        stats.Overall = new OverallStats
        {
            TotalItemsConsumed = completed.Sum(c => c.Count),
            TotalItemsInProgress = inProgress.Sum(i => i.Count),
            TotalTimeMs = sessions.TotalDurationMs,
            TotalSessions = sessions.SessionCount,
            MostActiveHour = hourly.FirstOrDefault()?.Hour,
            MostActiveDay = dow.FirstOrDefault()?.DayOfWeek
        };

        // Daily activity
        stats.DailyActivity = dailyCounts.Select(d => new DailyActivity
        {
            Date = DateTime.TryParse(d.Date, out var date) ? date : DateTime.MinValue,
            Sessions = d.Sessions,
            TotalTimeMs = d.TotalDurationMs
        }).ToList();

        _logger.LogDebug(
            "Generated consumption stats for user {UserId}: {Completed} completed, {Sessions} sessions, {TimeMs}ms total",
            userId, stats.Overall.TotalItemsConsumed, stats.Overall.TotalSessions, stats.Overall.TotalTimeMs);

        return stats;
    }

    public async Task<TasteProfile> GetTasteProfileAsync(string userId, CancellationToken ct = default)
    {
        var profile = new TasteProfile { UserId = userId };

        // Use all-time data for taste profile
        var period = DateRange.AllTime();

        var completed = await _repository.GetCompletedCountsByTypeAsync(userId, period, ct).ConfigureAwait(false);
        var sessions = await _repository.GetSessionAggregatesAsync(userId, period, ct).ConfigureAwait(false);
        var inProgress = await _repository.GetInProgressCountsByTypeAsync(userId, ct).ConfigureAwait(false);
        var dailyCounts = await _repository.GetDailySessionCountsAsync(userId, period, ct).ConfigureAwait(false);
        var hourly = await _repository.GetHourlyActivityAsync(userId, period, ct).ConfigureAwait(false);
        var dow = await _repository.GetDayOfWeekActivityAsync(userId, period, ct).ConfigureAwait(false);

        // Media preferences ranked by volume
        var totalCompleted = completed.Sum(c => c.Count);
        if (totalCompleted > 0)
        {
            profile.MediaPreferences = completed
                .OrderByDescending(c => c.Count)
                .Select(c => new MediaTypePreference
                {
                    MediaType = c.MediaType,
                    Score = (int)(c.Count * 100.0 / totalCompleted),
                    ItemCount = c.Count
                })
                .ToList();
        }

        // Completion rates
        foreach (var comp in completed)
        {
            var ip = inProgress.FirstOrDefault(i => i.MediaType == comp.MediaType);
            var total = comp.Count + (ip?.Count ?? 0);
            if (total > 0)
            {
                profile.CompletionRates[comp.MediaType] = (decimal)comp.Count / total;
            }
        }

        // Consumption pattern
        var sessionAgg = sessions.FirstOrDefault() ?? new SessionAggregate();
        var activeDays = dailyCounts.Count;
        var totalDays = Math.Max(1, activeDays);

        profile.Pattern = new ConsumptionPattern
        {
            ItemsPerWeek = activeDays > 0
                ? (decimal)totalCompleted / Math.Max(1, activeDays / 7.0m)
                : 0,
            AvgDailyTimeMs = dailyCounts.Count > 0
                ? dailyCounts.Sum(d => d.TotalDurationMs) / dailyCounts.Count
                : 0,
            PeakHour = hourly.FirstOrDefault()?.Hour ?? 0,
            PeakDay = dow.FirstOrDefault()?.DayOfWeek ?? 0,
            Style = DetermineStyle(sessionAgg, dailyCounts)
        };

        _logger.LogDebug(
            "Generated taste profile for user {UserId}: {TypeCount} media types, {Style} pattern",
            userId, profile.MediaPreferences.Count, profile.Pattern.Style);

        return profile;
    }

    private static ConsumptionStyle DetermineStyle(SessionAggregate sessions, List<DailySessionCount> daily)
    {
        if (sessions.SessionCount == 0 || daily.Count == 0)
            return ConsumptionStyle.Unknown;

        var avgSessionsPerDay = (decimal)sessions.SessionCount / Math.Max(1, daily.Count);
        var avgSessionMs = sessions.AvgDurationMs;

        // Binge: >3 sessions/day or avg session >4 hours
        if (avgSessionsPerDay > 3 || avgSessionMs > 4 * 60 * 60 * 1000)
            return ConsumptionStyle.Binge;

        // Calculate consistency: what percentage of days in the range have activity?
        if (daily.Count >= 2)
        {
            var dateRange = DateTime.TryParse(daily.Last().Date, out var last)
                && DateTime.TryParse(daily.First().Date, out var first)
                ? (last - first).TotalDays
                : 0;

            if (dateRange > 0)
            {
                var consistency = daily.Count / dateRange;
                if (consistency > 0.5) return ConsumptionStyle.Steady;
                return ConsumptionStyle.Sporadic;
            }
        }

        return ConsumptionStyle.Steady;
    }
}
