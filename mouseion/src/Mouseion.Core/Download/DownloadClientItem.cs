// Copyright (C) 2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Download;

public class DownloadClientItem
{
    public string DownloadId { get; set; } = string.Empty;
    public string Category { get; set; } = string.Empty;
    public string Title { get; set; } = string.Empty;
    public long TotalSize { get; set; }
    public long RemainingSize { get; set; }
    public TimeSpan? RemainingTime { get; set; }
    public double? SeedRatio { get; set; }
    public string OutputPath { get; set; } = string.Empty;
    public string Message { get; set; } = string.Empty;
    public DownloadItemStatus Status { get; set; }
    public bool IsEncrypted { get; set; }
    public bool CanMoveFiles { get; set; }
    public bool CanBeRemoved { get; set; }
    public bool Removed { get; set; }
}
