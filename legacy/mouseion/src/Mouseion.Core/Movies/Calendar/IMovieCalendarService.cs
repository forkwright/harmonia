// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Movies.Calendar;

public interface IMovieCalendarService
{
    Task<List<MovieCalendarEntry>> GetCalendarEntriesAsync(DateTime start, DateTime end, bool includeUnmonitored = false, CancellationToken ct = default);
    List<MovieCalendarEntry> GetCalendarEntries(DateTime start, DateTime end, bool includeUnmonitored = false);
}
