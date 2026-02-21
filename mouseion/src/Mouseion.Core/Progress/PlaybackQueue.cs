// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Progress;

public class PlaybackQueue : ModelBase
{
    public int UserId { get; set; }
    public string DeviceName { get; set; } = string.Empty;
    public string QueueData { get; set; } = "[]";
    public int CurrentIndex { get; set; }
    public bool ShuffleEnabled { get; set; }
    public string RepeatMode { get; set; } = "none";
    public DateTime UpdatedAt { get; set; }
}

public class QueueItem
{
    public int MediaItemId { get; set; }
    public int? MediaFileId { get; set; }
    public string Title { get; set; } = string.Empty;
    public string MediaType { get; set; } = string.Empty;
    public long? StartPositionMs { get; set; }
}
