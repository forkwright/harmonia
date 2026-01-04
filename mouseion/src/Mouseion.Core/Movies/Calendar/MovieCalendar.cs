// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Movies.Calendar;

public class MovieCalendarEntry
{
    public int MovieId { get; set; }
    public string Title { get; set; } = null!;
    public int Year { get; set; }
    public DateTime? ReleaseDate { get; set; }
    public string? ReleaseType { get; set; }
    public bool Monitored { get; set; }
    public bool HasFile { get; set; }
}
