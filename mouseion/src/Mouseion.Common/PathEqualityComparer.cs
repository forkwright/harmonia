// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Generic;
using System.IO;
using Mouseion.Common.EnvironmentInfo;

namespace Mouseion.Common
{
    public class PathEqualityComparer : IEqualityComparer<string>
    {
        public static readonly PathEqualityComparer Instance = new PathEqualityComparer();

        private PathEqualityComparer()
        {
        }

        public bool Equals(string? x, string? y)
        {
            if (x == null || y == null)
            {
                return x == y;
            }

            var comparison = OsInfo.IsWindows
                ? StringComparison.OrdinalIgnoreCase
                : StringComparison.Ordinal;

            var cleanX = CleanPath(x);
            var cleanY = CleanPath(y);

            return string.Equals(cleanX, cleanY, comparison);
        }

        public int GetHashCode(string obj)
        {
            var cleanPath = CleanPath(obj).Normalize();

            if (OsInfo.IsWindows)
            {
                return cleanPath.ToLower().GetHashCode();
            }

            return cleanPath.GetHashCode();
        }

        private static string CleanPath(string path)
        {
            if (string.IsNullOrWhiteSpace(path))
            {
                return path;
            }

            path = path.Trim().TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
            return Path.GetFullPath(path);
        }
    }
}
