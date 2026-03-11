// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.History;

public class MediaItemHistoryRepository : BasicRepository<MediaItemHistory>, IMediaItemHistoryRepository
{
    public MediaItemHistoryRepository(IDatabase database)
        : base(database, "History")
    {
    }

    public async Task<List<MediaItemHistory>> GetByMediaItemIdAsync(int mediaItemId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<MediaItemHistory>(
            "SELECT * FROM \"History\" WHERE \"MediaItemId\" = @MediaItemId ORDER BY \"Date\" DESC",
            new { MediaItemId = mediaItemId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<MediaItemHistory> GetByMediaItemId(int mediaItemId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<MediaItemHistory>(
            "SELECT * FROM \"History\" WHERE \"MediaItemId\" = @MediaItemId ORDER BY \"Date\" DESC",
            new { MediaItemId = mediaItemId }).ToList();
    }

    public async Task<List<MediaItemHistory>> FindByDownloadIdAsync(string downloadId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<MediaItemHistory>(
            "SELECT * FROM \"History\" WHERE \"DownloadId\" = @DownloadId ORDER BY \"Date\" DESC",
            new { DownloadId = downloadId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<MediaItemHistory> FindByDownloadId(string downloadId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<MediaItemHistory>(
            "SELECT * FROM \"History\" WHERE \"DownloadId\" = @DownloadId ORDER BY \"Date\" DESC",
            new { DownloadId = downloadId }).ToList();
    }

    public async Task<MediaItemHistory?> MostRecentForMediaItemAsync(int mediaItemId, HistoryEventType eventType, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleOrDefaultAsync<MediaItemHistory>(
            "SELECT * FROM \"History\" WHERE \"MediaItemId\" = @MediaItemId AND \"EventType\" = @EventType ORDER BY \"Date\" DESC LIMIT 1",
            new { MediaItemId = mediaItemId, EventType = (int)eventType }).ConfigureAwait(false);
    }

    public MediaItemHistory? MostRecentForMediaItem(int mediaItemId, HistoryEventType eventType)
    {
        using var conn = _database.OpenConnection();
        return conn.QuerySingleOrDefault<MediaItemHistory>(
            "SELECT * FROM \"History\" WHERE \"MediaItemId\" = @MediaItemId AND \"EventType\" = @EventType ORDER BY \"Date\" DESC LIMIT 1",
            new { MediaItemId = mediaItemId, EventType = (int)eventType });
    }

    public async Task<List<MediaItemHistory>> SinceAsync(DateTime date, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<MediaItemHistory>(
            "SELECT * FROM \"History\" WHERE \"Date\" >= @Date ORDER BY \"Date\" DESC",
            new { Date = date }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<MediaItemHistory> Since(DateTime date)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<MediaItemHistory>(
            "SELECT * FROM \"History\" WHERE \"Date\" >= @Date ORDER BY \"Date\" DESC",
            new { Date = date }).ToList();
    }

    public async Task<List<MediaItemHistory>> GetPagedAsync(int page, int pageSize, string? sortKey, string? sortDir, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var offset = (page - 1) * pageSize;
        var orderBy = BuildOrderByClause(sortKey, sortDir);

        var result = await conn.QueryAsync<MediaItemHistory>(
            $"SELECT * FROM \"History\" {orderBy} LIMIT @PageSize OFFSET @Offset",
            new { PageSize = pageSize, Offset = offset }).ConfigureAwait(false);
        return result.ToList();
    }

    private static string BuildOrderByClause(string? sortKey, string? sortDir)
    {
        var validSortKeys = new[] { "date", "eventType", "sourceTitle", "quality", "mediaItemId" };
        var key = sortKey?.ToLowerInvariant() ?? "date";

        if (!validSortKeys.Contains(key))
        {
            key = "date";
        }

        var column = key switch
        {
            "date" => "Date",
            "eventtype" => "EventType",
            "sourcetitle" => "SourceTitle",
            "quality" => "Quality",
            "mediaitemid" => "MediaItemId",
            _ => "Date"
        };

        var direction = sortDir?.ToUpperInvariant() == "ASC" ? "ASC" : "DESC";
        return $"ORDER BY \"{column}\" {direction}";
    }
}
