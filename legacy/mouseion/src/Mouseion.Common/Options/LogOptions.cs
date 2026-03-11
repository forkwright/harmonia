// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.Options
{
    public class LogOptions
    {
        public string? Level { get; set; }
        public bool? FilterSentryEvents { get; set; }
        public int? Rotate { get; set; }
        public int? SizeLimit { get; set; }
        public bool? Sql { get; set; }
        public string? ConsoleLevel { get; set; }
        public string? ConsoleFormat { get; set; }
        public bool? AnalyticsEnabled { get; set; }
        public string? SyslogServer { get; set; }
        public int? SyslogPort { get; set; }
        public string? SyslogLevel { get; set; }
        public bool? DbEnabled { get; set; }
    }
}
