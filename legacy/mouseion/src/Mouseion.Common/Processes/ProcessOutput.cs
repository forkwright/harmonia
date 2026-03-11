// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Collections.Generic;
using System.Linq;

namespace Mouseion.Common.Processes
{
    public class ProcessOutput
    {
        public int ExitCode { get; set; }
        public List<ProcessOutputLine> Lines { get; set; }

        public ProcessOutput()
        {
            Lines = new List<ProcessOutputLine>();
        }

        public List<ProcessOutputLine> Standard
        {
            get
            {
                return Lines.Where(c => c.Level == ProcessOutputLevel.Standard).ToList();
            }
        }

        public List<ProcessOutputLine> Error
        {
            get
            {
                return Lines.Where(c => c.Level == ProcessOutputLevel.Error).ToList();
            }
        }
    }
}
