// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Indexers.RateLimiting;

/// <summary>
/// Huntarr-inspired indexer rate limiter. Sliding window per-indexer budgets
/// with exponential backoff on errors. Prevents tracker bans by treating
/// indexers as resources with capacity, not unlimited endpoints.
/// </summary>
public class IndexerRateLimiter : IIndexerRateLimiter
{
    private readonly IIndexerRateLimitRepository _rateLimitRepo;
    private readonly IIndexerRequestLogRepository _requestLogRepo;
    private readonly ILogger<IndexerRateLimiter> _logger;

    // Backoff escalation: 1min → 5min → 30min → 4hr
    private static readonly TimeSpan[] BackoffStages = new[]
    {
        TimeSpan.FromMinutes(1),
        TimeSpan.FromMinutes(5),
        TimeSpan.FromMinutes(30),
        TimeSpan.FromHours(4)
    };

    public IndexerRateLimiter(
        IIndexerRateLimitRepository rateLimitRepo,
        IIndexerRequestLogRepository requestLogRepo,
        ILogger<IndexerRateLimiter> logger)
    {
        _rateLimitRepo = rateLimitRepo;
        _requestLogRepo = requestLogRepo;
        _logger = logger;
    }

    public (bool Allowed, string? Reason) CanRequest(string indexerName)
    {
        var config = _rateLimitRepo.GetByName(indexerName);

        if (config == null)
        {
            // No config = no limits (first use — will be configured on first request)
            return (true, null);
        }

        if (!config.Enabled)
        {
            return (false, $"Indexer '{indexerName}' is disabled");
        }

        // Check backoff
        if (config.BackoffUntil.HasValue && config.BackoffUntil.Value > DateTime.UtcNow)
        {
            var remaining = config.BackoffUntil.Value - DateTime.UtcNow;
            return (false, $"Indexer '{indexerName}' in backoff until {config.BackoffUntil:HH:mm:ss} ({remaining.TotalMinutes:F0}min remaining)");
        }

        // Check sliding window
        var windowStart = DateTime.UtcNow.AddHours(-1);
        var requestCount = _requestLogRepo.CountSince(indexerName, windowStart);

        if (requestCount >= config.MaxRequestsPerHour)
        {
            return (false, $"Indexer '{indexerName}' at capacity: {requestCount}/{config.MaxRequestsPerHour} requests in last hour");
        }

        return (true, null);
    }

    public void RecordRequest(string indexerName, int? responseCode = null, int? responseTimeMs = null,
        int? resultCount = null, string? searchQuery = null)
    {
        _requestLogRepo.Log(new IndexerRequestLog
        {
            IndexerName = indexerName,
            ResponseCode = responseCode,
            ResponseTimeMs = responseTimeMs,
            ResultCount = resultCount,
            SearchQuery = searchQuery
        });

        // Ensure config exists
        EnsureConfig(indexerName);

        // Clear backoff on successful request (error recovery)
        if (responseCode is null or (>= 200 and < 400))
        {
            var config = _rateLimitRepo.GetByName(indexerName);
            if (config?.BackoffMultiplier > 1)
            {
                config.BackoffMultiplier = 1;
                config.BackoffUntil = null;
                _rateLimitRepo.Upsert(config);
                _logger.LogInformation("Cleared backoff for indexer '{IndexerName}' after successful request", indexerName);
            }
        }
    }

    public void RecordError(string indexerName, int errorCode, string? message = null)
    {
        var config = EnsureConfig(indexerName);

        // Determine backoff stage from current multiplier
        var stageIndex = Math.Min(config.BackoffMultiplier - 1, BackoffStages.Length - 1);
        stageIndex = Math.Max(0, stageIndex);
        var backoffDuration = BackoffStages[stageIndex];

        config.BackoffUntil = DateTime.UtcNow.Add(backoffDuration);
        config.BackoffMultiplier = Math.Min(config.BackoffMultiplier + 1, BackoffStages.Length);
        config.LastErrorCode = errorCode;
        config.LastErrorMessage = message;
        config.LastErrorAt = DateTime.UtcNow;

        _rateLimitRepo.Upsert(config);

        _logger.LogWarning(
            "Indexer '{IndexerName}' error {ErrorCode}: {Message}. Backoff stage {Stage} ({Duration})",
            indexerName, errorCode, message ?? "unknown", stageIndex + 1, backoffDuration);

        // Also log the request
        _requestLogRepo.Log(new IndexerRequestLog
        {
            IndexerName = indexerName,
            ResponseCode = errorCode,
            SearchQuery = $"ERROR: {message}"
        });
    }

    public void ClearBackoff(string indexerName)
    {
        var config = _rateLimitRepo.GetByName(indexerName);
        if (config != null)
        {
            config.BackoffUntil = null;
            config.BackoffMultiplier = 1;
            config.LastErrorCode = null;
            config.LastErrorMessage = null;
            _rateLimitRepo.Upsert(config);
            _logger.LogInformation("Manually cleared backoff for indexer '{IndexerName}'", indexerName);
        }
    }

    public List<IndexerHealthStatus> GetHealthStatus()
    {
        var configs = _rateLimitRepo.GetAll();
        var windowStart = DateTime.UtcNow.AddHours(-1);

        return configs.Select(config =>
        {
            var requestCount = _requestLogRepo.CountSince(config.IndexerName, windowStart);

            return new IndexerHealthStatus
            {
                IndexerName = config.IndexerName,
                RequestsUsed = requestCount,
                RequestsRemaining = Math.Max(0, config.MaxRequestsPerHour - requestCount),
                MaxRequestsPerHour = config.MaxRequestsPerHour,
                IsInBackoff = config.BackoffUntil.HasValue && config.BackoffUntil.Value > DateTime.UtcNow,
                BackoffUntil = config.BackoffUntil,
                LastErrorCode = config.LastErrorCode,
                LastErrorMessage = config.LastErrorMessage,
                LastErrorAt = config.LastErrorAt,
                Enabled = config.Enabled
            };
        }).ToList();
    }

    public void Configure(string indexerName, int maxRequestsPerHour)
    {
        var config = EnsureConfig(indexerName);
        config.MaxRequestsPerHour = maxRequestsPerHour;
        _rateLimitRepo.Upsert(config);
        _logger.LogInformation("Configured rate limit for '{IndexerName}': {MaxReqs}/hour",
            indexerName, maxRequestsPerHour);
    }

    private IndexerRateLimit EnsureConfig(string indexerName)
    {
        var config = _rateLimitRepo.GetByName(indexerName);
        if (config == null)
        {
            config = new IndexerRateLimit
            {
                IndexerName = indexerName,
                MaxRequestsPerHour = 100,
                Enabled = true
            };
            config = _rateLimitRepo.Upsert(config);
        }
        return config;
    }
}
