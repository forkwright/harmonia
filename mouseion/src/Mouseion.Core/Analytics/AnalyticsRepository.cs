// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Analytics;

public interface IAnalyticsRepository
{
    Task<List<MediaTypeCount>> GetCompletedCountsByTypeAsync(string userId, DateRange period, CancellationToken ct = default);
    Task<List<MediaTypeCount>> GetInProgressCountsByTypeAsync(string userId, CancellationToken ct = default);
    Task<List<MediaTypeCount>> GetTotalCountsByTypeAsync(CancellationToken ct = default);
    Task<List<SessionAggregate>> GetSessionAggregatesAsync(string userId, DateRange period, CancellationToken ct = default);
    Task<List<DailySessionCount>> GetDailySessionCountsAsync(string userId, DateRange period, CancellationToken ct = default);
    Task<List<HourlyActivity>> GetHourlyActivityAsync(string userId, DateRange period, CancellationToken ct = default);
    Task<List<DayOfWeekActivity>> GetDayOfWeekActivityAsync(string userId, DateRange period, CancellationToken ct = default);
    Task<List<RatingAggregate>> GetAverageRatingsByTypeAsync(string userId, CancellationToken ct = default);
}

public class AnalyticsRepository : IAnalyticsRepository
{
    private readonly IDatabase _database;

    public AnalyticsRepository(IDatabase database)
    {
        _database = database;
    }

    public async Task<List<MediaTypeCount>> GetCompletedCountsByTypeAsync(string userId, DateRange period, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        // Join MediaProgress (completed items) with their media items to get type breakdown
        var result = await conn.QueryAsync<MediaTypeCount>(@"
            SELECT mi.""MediaType"", COUNT(*) as ""Count""
            FROM ""MediaProgress"" mp
            INNER JOIN ""Movies"" mi ON mp.""MediaItemId"" = mi.""Id""
            WHERE mp.""UserId"" = @UserId
              AND mp.""IsComplete"" = 1
              AND (@StartDate IS NULL OR mp.""UpdatedAt"" >= @StartDate)
              AND mp.""UpdatedAt"" <= @EndDate
            GROUP BY mi.""MediaType""

            UNION ALL

            SELECT mi.""MediaType"", COUNT(*) as ""Count""
            FROM ""MediaProgress"" mp
            INNER JOIN ""Books"" mi ON mp.""MediaItemId"" = mi.""Id""
            WHERE mp.""UserId"" = @UserId
              AND mp.""IsComplete"" = 1
              AND (@StartDate IS NULL OR mp.""UpdatedAt"" >= @StartDate)
              AND mp.""UpdatedAt"" <= @EndDate
            GROUP BY mi.""MediaType""

            UNION ALL

            SELECT mi.""MediaType"", COUNT(*) as ""Count""
            FROM ""MediaProgress"" mp
            INNER JOIN ""Audiobooks"" mi ON mp.""MediaItemId"" = mi.""Id""
            WHERE mp.""UserId"" = @UserId
              AND mp.""IsComplete"" = 1
              AND (@StartDate IS NULL OR mp.""UpdatedAt"" >= @StartDate)
              AND mp.""UpdatedAt"" <= @EndDate
            GROUP BY mi.""MediaType""",
            new
            {
                UserId = userId,
                StartDate = period.Start == DateTime.MinValue ? (DateTime?)null : period.Start,
                EndDate = period.End
            }).ConfigureAwait(false);

        return result.ToList();
    }

    public async Task<List<MediaTypeCount>> GetInProgressCountsByTypeAsync(string userId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        // Get in-progress items grouped by their media type
        // Since MediaProgress doesn't store MediaType, we aggregate across known tables
        var result = await conn.QueryAsync<MediaTypeCount>(@"
            SELECT @MovieType as ""MediaType"", COUNT(*) as ""Count""
            FROM ""MediaProgress"" mp
            INNER JOIN ""Movies"" mi ON mp.""MediaItemId"" = mi.""Id""
            WHERE mp.""UserId"" = @UserId AND mp.""IsComplete"" = 0

            UNION ALL

            SELECT @BookType as ""MediaType"", COUNT(*) as ""Count""
            FROM ""MediaProgress"" mp
            INNER JOIN ""Books"" mi ON mp.""MediaItemId"" = mi.""Id""
            WHERE mp.""UserId"" = @UserId AND mp.""IsComplete"" = 0

            UNION ALL

            SELECT @AudiobookType as ""MediaType"", COUNT(*) as ""Count""
            FROM ""MediaProgress"" mp
            INNER JOIN ""Audiobooks"" mi ON mp.""MediaItemId"" = mi.""Id""
            WHERE mp.""UserId"" = @UserId AND mp.""IsComplete"" = 0",
            new
            {
                UserId = userId,
                MovieType = (int)MediaType.Movie,
                BookType = (int)MediaType.Book,
                AudiobookType = (int)MediaType.Audiobook
            }).ConfigureAwait(false);

        return result.Where(r => r.Count > 0).ToList();
    }

    public async Task<List<MediaTypeCount>> GetTotalCountsByTypeAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        var result = await conn.QueryAsync<MediaTypeCount>(@"
            SELECT ""MediaType"", COUNT(*) as ""Count""
            FROM ""Movies"" GROUP BY ""MediaType""
            UNION ALL
            SELECT ""MediaType"", COUNT(*) as ""Count""
            FROM ""Books"" GROUP BY ""MediaType""
            UNION ALL
            SELECT ""MediaType"", COUNT(*) as ""Count""
            FROM ""Audiobooks"" GROUP BY ""MediaType""").ConfigureAwait(false);

        return result.ToList();
    }

    public async Task<List<SessionAggregate>> GetSessionAggregatesAsync(string userId, DateRange period, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        var result = await conn.QueryAsync<SessionAggregate>(@"
            SELECT
                COUNT(*) as ""SessionCount"",
                SUM(""DurationMs"") as ""TotalDurationMs"",
                AVG(""DurationMs"") as ""AvgDurationMs"",
                MAX(""DurationMs"") as ""MaxDurationMs""
            FROM ""PlaybackSessions""
            WHERE ""UserId"" = @UserId
              AND ""IsActive"" = 0
              AND (@StartDate IS NULL OR ""StartedAt"" >= @StartDate)
              AND ""StartedAt"" <= @EndDate",
            new
            {
                UserId = userId,
                StartDate = period.Start == DateTime.MinValue ? (DateTime?)null : period.Start,
                EndDate = period.End
            }).ConfigureAwait(false);

        return result.ToList();
    }

    public async Task<List<DailySessionCount>> GetDailySessionCountsAsync(string userId, DateRange period, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        var result = await conn.QueryAsync<DailySessionCount>(@"
            SELECT
                DATE(""StartedAt"") as ""Date"",
                COUNT(*) as ""Sessions"",
                SUM(""DurationMs"") as ""TotalDurationMs""
            FROM ""PlaybackSessions""
            WHERE ""UserId"" = @UserId
              AND ""IsActive"" = 0
              AND (@StartDate IS NULL OR ""StartedAt"" >= @StartDate)
              AND ""StartedAt"" <= @EndDate
            GROUP BY DATE(""StartedAt"")
            ORDER BY DATE(""StartedAt"")",
            new
            {
                UserId = userId,
                StartDate = period.Start == DateTime.MinValue ? (DateTime?)null : period.Start,
                EndDate = period.End
            }).ConfigureAwait(false);

        return result.ToList();
    }

    public async Task<List<HourlyActivity>> GetHourlyActivityAsync(string userId, DateRange period, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        var result = await conn.QueryAsync<HourlyActivity>(@"
            SELECT
                CAST(strftime('%H', ""StartedAt"") AS INTEGER) as ""Hour"",
                COUNT(*) as ""Sessions"",
                SUM(""DurationMs"") as ""TotalDurationMs""
            FROM ""PlaybackSessions""
            WHERE ""UserId"" = @UserId
              AND ""IsActive"" = 0
              AND (@StartDate IS NULL OR ""StartedAt"" >= @StartDate)
              AND ""StartedAt"" <= @EndDate
            GROUP BY CAST(strftime('%H', ""StartedAt"") AS INTEGER)
            ORDER BY ""Sessions"" DESC",
            new
            {
                UserId = userId,
                StartDate = period.Start == DateTime.MinValue ? (DateTime?)null : period.Start,
                EndDate = period.End
            }).ConfigureAwait(false);

        return result.ToList();
    }

    public async Task<List<DayOfWeekActivity>> GetDayOfWeekActivityAsync(string userId, DateRange period, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        var result = await conn.QueryAsync<DayOfWeekActivity>(@"
            SELECT
                CAST(strftime('%w', ""StartedAt"") AS INTEGER) as ""DayOfWeek"",
                COUNT(*) as ""Sessions"",
                SUM(""DurationMs"") as ""TotalDurationMs""
            FROM ""PlaybackSessions""
            WHERE ""UserId"" = @UserId
              AND ""IsActive"" = 0
              AND (@StartDate IS NULL OR ""StartedAt"" >= @StartDate)
              AND ""StartedAt"" <= @EndDate
            GROUP BY CAST(strftime('%w', ""StartedAt"") AS INTEGER)
            ORDER BY ""Sessions"" DESC",
            new
            {
                UserId = userId,
                StartDate = period.Start == DateTime.MinValue ? (DateTime?)null : period.Start,
                EndDate = period.End
            }).ConfigureAwait(false);

        return result.ToList();
    }

    public async Task<List<RatingAggregate>> GetAverageRatingsByTypeAsync(string userId, CancellationToken ct = default)
    {
        // Ratings come from import list items (UserRating field)
        // This is a placeholder — when user rating storage is formalized,
        // this query will hit the proper ratings table
        return await Task.FromResult(new List<RatingAggregate>()).ConfigureAwait(false);
    }
}

// Query result DTOs
public class MediaTypeCount
{
    public MediaType MediaType { get; set; }
    public int Count { get; set; }
}

public class SessionAggregate
{
    public int SessionCount { get; set; }
    public long TotalDurationMs { get; set; }
    public long AvgDurationMs { get; set; }
    public long MaxDurationMs { get; set; }
}

public class DailySessionCount
{
    public string Date { get; set; } = string.Empty;
    public int Sessions { get; set; }
    public long TotalDurationMs { get; set; }
}

public class HourlyActivity
{
    public int Hour { get; set; }
    public int Sessions { get; set; }
    public long TotalDurationMs { get; set; }
}

public class DayOfWeekActivity
{
    public int DayOfWeek { get; set; }
    public int Sessions { get; set; }
    public long TotalDurationMs { get; set; }
}

public class RatingAggregate
{
    public MediaType MediaType { get; set; }
    public decimal AverageRating { get; set; }
    public int RatedCount { get; set; }
}
