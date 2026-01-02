// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;

namespace Mouseion.Common.EnvironmentInfo
{
    public interface IPlatformInfo
    {
        Version Version { get; }
    }

    public class PlatformInfo : IPlatformInfo
    {
        private static Version _version;

        static PlatformInfo()
        {
            _version = Environment.Version;
        }

        public static string PlatformName
        {
            get
            {
                return ".NET";
            }
        }

        public Version Version => _version;

        public static Version GetVersion()
        {
            return _version;
        }
    }
}
