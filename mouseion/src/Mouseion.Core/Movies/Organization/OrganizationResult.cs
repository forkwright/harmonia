// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Movies.Organization;

public class OrganizationResult
{
    public bool Success { get; set; }
    public string OriginalPath { get; set; } = string.Empty;
    public string NewPath { get; set; } = string.Empty;
    public string? ErrorMessage { get; set; }
    public FileStrategy StrategyUsed { get; set; }
    public bool IsDryRun { get; set; }
}
