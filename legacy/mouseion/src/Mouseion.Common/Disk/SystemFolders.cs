// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Generic;
using Mouseion.Common.EnvironmentInfo;

namespace Mouseion.Common.Disk
{
    public static class SystemFolders
    {
        public static List<string> GetSystemFolders()
        {
            if (OsInfo.IsWindows)
            {
                return new List<string> { Environment.GetFolderPath(Environment.SpecialFolder.Windows) };
            }

            if (OsInfo.IsOsx)
            {
                return new List<string> { "/System" };
            }

            return new List<string>
            {
                "/bin",
                "/boot",
                "/lib",
                "/sbin",
                "/proc",
                "/usr/bin"
            };
        }
    }
}
