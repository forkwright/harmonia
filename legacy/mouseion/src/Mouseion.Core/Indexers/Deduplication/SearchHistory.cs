// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Indexers.Deduplication;

public class SearchHistoryEntry : ModelBase
{
    public int MediaItemId { get; set; }
    public string MediaType { get; set; } = string.Empty;
    public string IndexerName { get; set; } = string.Empty;
    public string SearchQuery { get; set; } = string.Empty;
    public int ResultCount { get; set; }
    public string? BestMatchTitle { get; set; }
    public string? BestMatchGuid { get; set; }
    public DateTime SearchedAt { get; set; } = DateTime.UtcNow;
}

public class GrabbedRelease : ModelBase
{
    public int MediaItemId { get; set; }
    public string IndexerName { get; set; } = string.Empty;
    public string ReleaseGuid { get; set; } = string.Empty;
    public string ReleaseTitle { get; set; } = string.Empty;
    public string Quality { get; set; } = string.Empty;
    public long SizeBytes { get; set; }
    public string? DownloadClientId { get; set; }
    public DateTime GrabbedAt { get; set; } = DateTime.UtcNow;
}

public class SkippedRelease : ModelBase
{
    public int MediaItemId { get; set; }
    public string ReleaseGuid { get; set; } = string.Empty;
    public string ReleaseTitle { get; set; } = string.Empty;
    public string Reason { get; set; } = string.Empty;
    public DateTime SkippedAt { get; set; } = DateTime.UtcNow;
}
