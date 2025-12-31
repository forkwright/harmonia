// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.RegularExpressions;

namespace Mouseion.Common.Extensions
{
    public static class RegexExtensions
    {
        public static int EndIndex(this Capture regexMatch)
        {
            return regexMatch.Index + regexMatch.Length;
        }
    }
}
