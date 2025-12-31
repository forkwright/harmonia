// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Globalization;

namespace Mouseion.Common.Extensions
{
    public static class TryParseExtensions
    {
        public static int? ParseInt32(this string source)
        {
            if (int.TryParse(source, out var result))
            {
                return result;
            }

            return null;
        }

        public static long? ParseInt64(this string source)
        {
            if (long.TryParse(source, out var result))
            {
                return result;
            }

            return null;
        }

        public static double? ParseDouble(this string source)
        {
            if (double.TryParse(source.Replace(',', '.'), NumberStyles.Number, CultureInfo.InvariantCulture, out var result))
            {
                return result;
            }

            return null;
        }
    }
}
