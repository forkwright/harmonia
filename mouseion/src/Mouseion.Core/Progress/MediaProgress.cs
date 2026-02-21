// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Progress;

public class MediaProgress : ModelBase
{
    public int MediaItemId { get; set; }
    public string UserId { get; set; } = "default";
    public int UserIdInt { get; set; } = 1;
    public long PositionMs { get; set; }
    public long TotalDurationMs { get; set; }
    public decimal PercentComplete { get; set; }
    public DateTime LastPlayedAt { get; set; }
    public bool IsComplete { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
}
