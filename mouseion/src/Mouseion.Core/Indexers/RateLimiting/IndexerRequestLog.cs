// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Indexers.RateLimiting;

/// <summary>
/// Individual indexer request record for sliding window tracking.
/// </summary>
public class IndexerRequestLog : ModelBase
{
    public string IndexerName { get; set; } = string.Empty;
    public DateTime RequestedAt { get; set; }
    public int? ResponseCode { get; set; }
    public int? ResponseTimeMs { get; set; }
    public int? ResultCount { get; set; }
    public string? SearchQuery { get; set; }
}
