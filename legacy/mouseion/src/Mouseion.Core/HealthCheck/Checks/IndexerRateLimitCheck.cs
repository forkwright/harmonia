// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Indexers.RateLimiting;

namespace Mouseion.Core.HealthCheck.Checks;

/// <summary>
/// Health check that reports indexers at capacity or in backoff.
/// </summary>
public class IndexerRateLimitCheck : IProvideHealthCheck
{
    private readonly IIndexerRateLimiter _rateLimiter;

    public IndexerRateLimitCheck(IIndexerRateLimiter rateLimiter)
    {
        _rateLimiter = rateLimiter;
    }

    public HealthCheck Check()
    {
        var statuses = _rateLimiter.GetHealthStatus();

        if (statuses.Count == 0)
        {
            return new HealthCheck(HealthCheckResult.Ok, "No indexers configured", "indexer-rate-ok");
        }

        var inBackoff = statuses.Where(s => s.IsInBackoff).ToList();
        var atCapacity = statuses.Where(s => !s.IsInBackoff && s.RequestsRemaining == 0).ToList();

        if (inBackoff.Count > 0)
        {
            var names = string.Join(", ", inBackoff.Select(s => s.IndexerName));
            return new HealthCheck(
                HealthCheckResult.Warning,
                $"Indexers in backoff: {names}",
                "indexer-backoff");
        }

        if (atCapacity.Count > 0)
        {
            var names = string.Join(", ", atCapacity.Select(s => $"{s.IndexerName} ({s.RequestsUsed}/{s.MaxRequestsPerHour})"));
            return new HealthCheck(
                HealthCheckResult.Notice,
                $"Indexers at capacity: {names}",
                "indexer-capacity");
        }

        return new HealthCheck(HealthCheckResult.Ok,
            $"{statuses.Count} indexer(s) healthy",
            "indexer-rate-ok");
    }
}
