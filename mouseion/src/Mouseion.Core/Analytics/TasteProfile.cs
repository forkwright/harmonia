// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Analytics;

/// <summary>
/// User's taste profile derived from consumption history, ratings, and completion patterns.
/// Used for recommendations and smart list tuning.
/// </summary>
public class TasteProfile
{
    public string UserId { get; set; } = "default";
    public DateTime GeneratedAt { get; set; } = DateTime.UtcNow;

    /// <summary>
    /// Preferred media types ranked by consumption volume.
    /// </summary>
    public List<MediaTypePreference> MediaPreferences { get; set; } = new();

    /// <summary>
    /// Genre preferences with weighted scores from ratings + completion rates.
    /// </summary>
    public List<GenrePreference> GenrePreferences { get; set; } = new();

    /// <summary>
    /// Top artists/authors/directors by consumption volume and rating.
    /// </summary>
    public List<CreatorPreference> TopCreators { get; set; } = new();

    /// <summary>
    /// Consumption pattern analysis.
    /// </summary>
    public ConsumptionPattern Pattern { get; set; } = new();

    /// <summary>
    /// Completion rate by media type (how often does the user finish what they start).
    /// </summary>
    public Dictionary<MediaType, decimal> CompletionRates { get; set; } = new();
}

public class MediaTypePreference
{
    public MediaType MediaType { get; set; }

    /// <summary>
    /// Normalized score 0-100 based on relative consumption volume.
    /// </summary>
    public int Score { get; set; }

    /// <summary>
    /// Total items consumed in this media type.
    /// </summary>
    public int ItemCount { get; set; }

    /// <summary>
    /// Average rating given to items in this type (1-10 scale).
    /// </summary>
    public decimal? AverageRating { get; set; }
}

public class GenrePreference
{
    public string Genre { get; set; } = string.Empty;

    /// <summary>
    /// Weighted score: (items_rated * avg_rating) + (items_completed * completion_bonus).
    /// Normalized to 0-100.
    /// </summary>
    public int Score { get; set; }

    public int ItemCount { get; set; }
    public decimal? AverageRating { get; set; }
    public decimal CompletionRate { get; set; }
}

public class CreatorPreference
{
    public string Name { get; set; } = string.Empty;
    public MediaType MediaType { get; set; }
    public int ItemCount { get; set; }
    public decimal? AverageRating { get; set; }
    public long TotalTimeMs { get; set; }
}

public class ConsumptionPattern
{
    /// <summary>
    /// Average items consumed per week over the analysis period.
    /// </summary>
    public decimal ItemsPerWeek { get; set; }

    /// <summary>
    /// Average daily consumption time in milliseconds.
    /// </summary>
    public long AvgDailyTimeMs { get; set; }

    /// <summary>
    /// Day of week with highest activity (0=Sunday, 6=Saturday).
    /// </summary>
    public int PeakDay { get; set; }

    /// <summary>
    /// Hour of day with highest activity (0-23, UTC).
    /// </summary>
    public int PeakHour { get; set; }

    /// <summary>
    /// Whether the user tends to binge (multiple items/sessions in one sitting)
    /// vs. steady consumption (1 item per day spread out).
    /// </summary>
    public ConsumptionStyle Style { get; set; }

    /// <summary>
    /// Average number of days between starting and completing an item.
    /// </summary>
    public decimal? AvgDaysToComplete { get; set; }
}

public enum ConsumptionStyle
{
    Unknown,
    Binge,     // >3 sessions/day average or >4 hours/session
    Steady,    // 1-2 sessions/day, consistent across days
    Sporadic   // Irregular, long gaps between sessions
}
