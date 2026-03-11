// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Qualities;

namespace Mouseion.Core.History;

public class MediaItemHistory : ModelBase
{
    public int MediaItemId { get; set; }
    public MediaType MediaType { get; set; }
    public string SourceTitle { get; set; } = null!;
    public QualityModel Quality { get; set; } = null!;
    public DateTime Date { get; set; }
    public HistoryEventType EventType { get; set; }
    public string Data { get; set; } = "{}";
    public string? DownloadId { get; set; }
}

public enum HistoryEventType
{
    Unknown = 0,
    Grabbed = 1,
    DownloadFolderImported = 3,
    DownloadFailed = 4,
    MediaFileDeleted = 5,
    MediaFileRenamed = 6,
    DownloadIgnored = 7
}
