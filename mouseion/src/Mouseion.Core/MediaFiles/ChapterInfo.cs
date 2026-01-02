// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.MediaFiles;

public class ChapterInfo
{
    public int Index { get; set; }
    public string Title { get; set; } = string.Empty;
    public long StartTimeMs { get; set; }
    public long EndTimeMs { get; set; }

    public long DurationMs => EndTimeMs - StartTimeMs;
}
