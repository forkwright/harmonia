// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.EnvironmentInfo;

namespace Mouseion.Common.Http
{
    public interface IUserAgentBuilder
    {
        string GetUserAgent(bool simplified = false);
    }

    public class UserAgentBuilder : IUserAgentBuilder
    {
        private readonly string _userAgentSimplified;
        private readonly string _userAgent;

        public string GetUserAgent(bool simplified)
        {
            if (simplified)
            {
                return _userAgentSimplified;
            }

            return _userAgent;
        }

        public UserAgentBuilder(IOsInfo osInfo)
        {
            var osName = OsInfo.Os.ToString();

            if (!string.IsNullOrWhiteSpace(osInfo.Name))
            {
                osName = osInfo.Name.ToLower();
            }

            var osVersion = osInfo.Version?.ToLower();

            _userAgent = $"{BuildInfo.AppName}/{BuildInfo.Version} ({osName} {osVersion})";
            _userAgentSimplified = $"{BuildInfo.AppName}/{BuildInfo.Version.ToString(2)}";
        }
    }
}
