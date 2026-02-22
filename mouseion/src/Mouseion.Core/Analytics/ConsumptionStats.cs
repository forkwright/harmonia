// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Analytics;

/// <summary>
/// Aggregated consumption statistics across all media types.
/// </summary>
public class ConsumptionStats
{
    public string UserId { get; set; } = "default";
    public DateRange Period { get; set; } = new();
    public List<MediaTypeStats> ByMediaType { get; set; } = new();
    public OverallStats Overall { get; set; } = new();
    public List<DailyActivity> DailyActivity { get; set; } = new();
}

public class OverallStats
{
    public int TotalItemsConsumed { get; set; }
    public int TotalItemsInProgress { get; set; }
    public long TotalTimeMs { get; set; }
    public int TotalSessions { get; set; }

    /// <summary>
    /// Average session length in milliseconds.
    /// </summary>
    public long AverageSessionMs => TotalSessions > 0 ? TotalTimeMs / TotalSessions : 0;

    /// <summary>
    /// Most active day of the week (0=Sunday, 6=Saturday).
    /// </summary>
    public int? MostActiveDay { get; set; }

    /// <summary>
    /// Most active hour of day (0-23, UTC).
    /// </summary>
    public int? MostActiveHour { get; set; }
}

public class MediaTypeStats
{
    public MediaType MediaType { get; set; }
    public int ItemsCompleted { get; set; }
    public int ItemsInProgress { get; set; }
    public int ItemsTotal { get; set; }
    public long TotalTimeMs { get; set; }
    public int Sessions { get; set; }

    // Media-specific metrics
    /// <summary>Movies: hours watched. TV: episodes watched.</summary>
    public int? EpisodesWatched { get; set; }

    /// <summary>Books: estimated pages read (from percent * total pages when available).</summary>
    public int? PagesRead { get; set; }

    /// <summary>Music: total play count from scrobble imports.</summary>
    public int? PlayCount { get; set; }

    /// <summary>Podcasts: episodes listened to completion.</summary>
    public int? EpisodesListened { get; set; }

    /// <summary>Average user rating for completed items in this media type.</summary>
    public decimal? AverageRating { get; set; }
}

public class DailyActivity
{
    public DateTime Date { get; set; }
    public int Sessions { get; set; }
    public long TotalTimeMs { get; set; }
    public int ItemsCompleted { get; set; }
    public Dictionary<MediaType, int> SessionsByType { get; set; } = new();
}

public class DateRange
{
    public DateTime Start { get; set; }
    public DateTime End { get; set; }

    public static DateRange Last7Days() => new()
    {
        Start = DateTime.UtcNow.Date.AddDays(-7),
        End = DateTime.UtcNow
    };

    public static DateRange Last30Days() => new()
    {
        Start = DateTime.UtcNow.Date.AddDays(-30),
        End = DateTime.UtcNow
    };

    public static DateRange Last90Days() => new()
    {
        Start = DateTime.UtcNow.Date.AddDays(-90),
        End = DateTime.UtcNow
    };

    public static DateRange Last365Days() => new()
    {
        Start = DateTime.UtcNow.Date.AddDays(-365),
        End = DateTime.UtcNow
    };

    public static DateRange AllTime() => new()
    {
        Start = DateTime.MinValue,
        End = DateTime.UtcNow
    };
}
