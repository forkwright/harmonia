// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;

namespace Mouseion.Common.Processes
{
    public class ProcessOutputLine
    {
        public ProcessOutputLevel Level { get; set; }
        public string Content { get; set; }
        public DateTime Time { get; set; }

        public ProcessOutputLine(ProcessOutputLevel level, string content)
        {
            Level = level;
            Content = content;
            Time = DateTime.UtcNow;
        }

        public override string ToString()
        {
            return string.Format("{0} - {1} - {2}", Time, Level, Content);
        }
    }

    public enum ProcessOutputLevel
    {
        Standard = 0,
        Error = 1
    }
}
