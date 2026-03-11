// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Movies.Monitoring;

public interface IReleaseMonitoringService
{
    Task<List<Movie>> GetUpcomingReleasesAsync(int daysAhead = 30, CancellationToken ct = default);
    Task<List<Movie>> GetRecentReleasesAsync(int daysBehind = 7, CancellationToken ct = default);
}
