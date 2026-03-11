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
    public interface IRuntimeInfo
    {
        DateTime StartTime { get; }
        bool IsUserInteractive { get; }
        bool IsAdmin { get; }
        bool IsWindowsService { get; }
        bool IsWindowsTray { get; }
        bool IsStarting { get; set; }
        bool IsExiting { get; set; }
        bool IsTray { get; }
        RuntimeMode Mode { get; }
        bool RestartPending { get; set; }
        string ExecutingApplication { get; }
    }
}
