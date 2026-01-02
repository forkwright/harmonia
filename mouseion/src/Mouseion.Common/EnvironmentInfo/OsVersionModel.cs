// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.EnvironmentInfo
{
    public class OsVersionModel
    {
        public OsVersionModel(string name, string version, string fullName = null)
        {
            Name = Trim(name);
            Version = Trim(version);

            if (string.IsNullOrWhiteSpace(fullName))
            {
                fullName = $"{Name} {Version}";
            }

            FullName = Trim(fullName);
        }

        private static string Trim(string source)
        {
            return source.Trim().Trim('"', '\'');
        }

        public string Name { get; }
        public string FullName { get; }
        public string Version { get; }
    }
}
