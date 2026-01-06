// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Progress;

public class MediaProgress : ModelBase
{
    public int MediaItemId { get; set; }
    public string UserId { get; set; } = "default";
    public long PositionMs { get; set; }
    public long TotalDurationMs { get; set; }
    public decimal PercentComplete { get; set; }
    public DateTime LastPlayedAt { get; set; }
    public bool IsComplete { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
}
