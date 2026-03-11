// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Collections.Generic;

namespace Mouseion.Common.Disk
{
    public static class SpecialFolders
    {
        private static readonly HashSet<string> _specialFolders = new HashSet<string>
        {
            // Windows
            "boot",
            "bootmgr",
            "cache",
            "msocache",
            "recovery",
            "$recycle.bin",
            "recycler",
            "system volume information",
            "temporary internet files",
            "windows",

            // OS X
            ".fseventd",
            ".spotlight",
            ".trashes",
            ".vol",
            "cachedmessages",
            "caches",
            "trash",

            // QNAP
            ".@__thumb",

            // Synology
            "@eadir",
            "#recycle"
        };

        public static bool IsSpecialFolder(string? folder)
        {
            if (folder == null)
            {
                return false;
            }

            return _specialFolders.Contains(folder.ToLowerInvariant());
        }
    }
}
