// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Indexers.RateLimiting;

/// <summary>
/// Central rate limiter for all indexer requests. Wraps search calls with
/// sliding window tracking, exponential backoff, and per-indexer budgets.
/// </summary>
public interface IIndexerRateLimiter
{
    /// <summary>
    /// Check if an indexer has available capacity for a request.
    /// Returns (allowed, reason) — if not allowed, reason explains why.
    /// </summary>
    (bool Allowed, string? Reason) CanRequest(string indexerName);

    /// <summary>
    /// Record a successful request against an indexer's budget.
    /// </summary>
    void RecordRequest(string indexerName, int? responseCode = null, int? responseTimeMs = null,
        int? resultCount = null, string? searchQuery = null);

    /// <summary>
    /// Record an error and apply backoff. Backoff escalates:
    /// 1min → 5min → 30min → 4hr on consecutive errors.
    /// </summary>
    void RecordError(string indexerName, int errorCode, string? message = null);

    /// <summary>
    /// Clear backoff state for an indexer (e.g., after manual reset).
    /// </summary>
    void ClearBackoff(string indexerName);

    /// <summary>
    /// Get current health status for all configured indexers.
    /// </summary>
    List<IndexerHealthStatus> GetHealthStatus();

    /// <summary>
    /// Update rate limit configuration for an indexer.
    /// </summary>
    void Configure(string indexerName, int maxRequestsPerHour);
}

public class IndexerHealthStatus
{
    public string IndexerName { get; set; } = string.Empty;
    public int RequestsUsed { get; set; }
    public int RequestsRemaining { get; set; }
    public int MaxRequestsPerHour { get; set; }
    public bool IsInBackoff { get; set; }
    public DateTime? BackoffUntil { get; set; }
    public int? LastErrorCode { get; set; }
    public string? LastErrorMessage { get; set; }
    public DateTime? LastErrorAt { get; set; }
    public bool Enabled { get; set; }
}
