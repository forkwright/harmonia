// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.Progress;

public class PlaybackSession : ModelBase
{
    public string SessionId { get; set; } = Guid.NewGuid().ToString();
    public int MediaItemId { get; set; }
    public string UserId { get; set; } = "default";
    public string DeviceName { get; set; } = string.Empty;
    public string DeviceType { get; set; } = string.Empty;
    public DateTime StartedAt { get; set; }
    public DateTime? EndedAt { get; set; }
    public long StartPositionMs { get; set; }
    public long? EndPositionMs { get; set; }
    public long DurationMs { get; set; }
    public bool IsActive { get; set; }
}
