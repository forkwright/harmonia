// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Indexers.RateLimiting;

/// <summary>
/// Per-indexer rate limit configuration and backoff state.
/// </summary>
public class IndexerRateLimit : ModelBase
{
    public string IndexerName { get; set; } = string.Empty;
    public int MaxRequestsPerHour { get; set; } = 100;
    public DateTime? BackoffUntil { get; set; }
    public int BackoffMultiplier { get; set; } = 1;
    public int? LastErrorCode { get; set; }
    public string? LastErrorMessage { get; set; }
    public DateTime? LastErrorAt { get; set; }
    public bool Enabled { get; set; } = true;
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
}
