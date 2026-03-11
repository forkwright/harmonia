// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.MediaCovers;

public class MediaCoversUpdatedEvent
{
    public int MediaItemId { get; set; }
    public bool Updated { get; set; }

    public MediaCoversUpdatedEvent(int mediaItemId, bool updated)
    {
        MediaItemId = mediaItemId;
        Updated = updated;
    }
}
