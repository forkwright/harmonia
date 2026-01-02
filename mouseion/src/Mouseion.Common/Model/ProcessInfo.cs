// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.Model
{
    public class ProcessInfo
    {
        public int Id { get; set; }
        public string? Name { get; set; }
        public string? StartPath { get; set; }

        public override string ToString()
        {
            return string.Format("{0}:{1} [{2}]", Id, Name ?? "Unknown", StartPath ?? "Unknown");
        }
    }
}
