// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.Http
{
    public static class UserAgentParser
    {
        public static string SimplifyUserAgent(string userAgent)
        {
            if (userAgent == null || userAgent.StartsWith("Mozilla/5.0"))
            {
                return null;
            }

            return userAgent;
        }
    }
}
