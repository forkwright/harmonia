// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.News;

namespace Mouseion.Core.HealthCheck.Checks;

public class NewsFeedHealthCheck : IProvideHealthCheck
{
    private readonly INewsFeedRepository _feedRepository;
    private static readonly TimeSpan StaleThreshold = TimeSpan.FromHours(24);

    public NewsFeedHealthCheck(INewsFeedRepository feedRepository)
    {
        _feedRepository = feedRepository;
    }

    public HealthCheck Check()
    {
        var monitoredFeeds = _feedRepository.GetMonitored();

        if (monitoredFeeds.Count == 0)
        {
            return new HealthCheck(
                HealthCheckResult.Ok,
                "No news feeds configured"
            );
        }

        var staleFeeds = monitoredFeeds
            .Where(f => f.LastFetchTime.HasValue &&
                        DateTime.UtcNow - f.LastFetchTime.Value > StaleThreshold)
            .ToList();

        var neverFetchedFeeds = monitoredFeeds
            .Where(f => !f.LastFetchTime.HasValue)
            .ToList();

        if (neverFetchedFeeds.Count > 0)
        {
            return new HealthCheck(
                HealthCheckResult.Warning,
                $"{neverFetchedFeeds.Count} feed(s) have never been fetched",
                "news-feed-never-fetched"
            );
        }

        if (staleFeeds.Count > 0)
        {
            return new HealthCheck(
                HealthCheckResult.Warning,
                $"{staleFeeds.Count} feed(s) have not been refreshed in over 24 hours",
                "news-feed-stale"
            );
        }

        return new HealthCheck(
            HealthCheckResult.Ok,
            $"All {monitoredFeeds.Count} news feed(s) are up to date"
        );
    }
}
