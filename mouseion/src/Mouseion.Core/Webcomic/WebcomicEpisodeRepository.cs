// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Webcomic;

public interface IWebcomicEpisodeRepository : IBasicRepository<WebcomicEpisode>
{
    Task<List<WebcomicEpisode>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default);
    Task<WebcomicEpisode?> FindByExternalIdAsync(string externalId, CancellationToken ct = default);
    Task<WebcomicEpisode?> FindByEpisodeNumberAsync(int seriesId, int episodeNumber, CancellationToken ct = default);
    Task<List<WebcomicEpisode>> GetUnreadAsync(CancellationToken ct = default);
    Task<List<WebcomicEpisode>> GetUnreadBySeriesAsync(int seriesId, CancellationToken ct = default);
    Task<int> GetUnreadCountAsync(CancellationToken ct = default);
    Task<int> GetUnreadCountBySeriesAsync(int seriesId, CancellationToken ct = default);
    Task MarkReadAsync(int id, CancellationToken ct = default);
    Task MarkUnreadAsync(int id, CancellationToken ct = default);
    Task MarkAllReadBySeriesAsync(int seriesId, CancellationToken ct = default);

    List<WebcomicEpisode> GetBySeriesId(int seriesId);
    WebcomicEpisode? FindByExternalId(string externalId);
    List<WebcomicEpisode> GetUnread();
    int GetUnreadCount();
}

public class WebcomicEpisodeRepository : BasicRepository<WebcomicEpisode>, IWebcomicEpisodeRepository
{
    public WebcomicEpisodeRepository(IDatabase database)
        : base(database, "WebcomicEpisodes")
    {
    }

    public async Task<List<WebcomicEpisode>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<WebcomicEpisode>(
            "SELECT * FROM \"WebcomicEpisodes\" WHERE \"WebcomicSeriesId\" = @SeriesId ORDER BY \"EpisodeNumber\" DESC",
            new { SeriesId = seriesId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<WebcomicEpisode> GetBySeriesId(int seriesId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<WebcomicEpisode>(
            "SELECT * FROM \"WebcomicEpisodes\" WHERE \"WebcomicSeriesId\" = @SeriesId ORDER BY \"EpisodeNumber\" DESC",
            new { SeriesId = seriesId }).ToList();
    }

    public async Task<WebcomicEpisode?> FindByExternalIdAsync(string externalId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<WebcomicEpisode>(
            "SELECT * FROM \"WebcomicEpisodes\" WHERE \"ExternalId\" = @ExternalId",
            new { ExternalId = externalId }).ConfigureAwait(false);
    }

    public WebcomicEpisode? FindByExternalId(string externalId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<WebcomicEpisode>(
            "SELECT * FROM \"WebcomicEpisodes\" WHERE \"ExternalId\" = @ExternalId",
            new { ExternalId = externalId });
    }

    public async Task<WebcomicEpisode?> FindByEpisodeNumberAsync(int seriesId, int episodeNumber, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<WebcomicEpisode>(
            "SELECT * FROM \"WebcomicEpisodes\" WHERE \"WebcomicSeriesId\" = @SeriesId AND \"EpisodeNumber\" = @EpisodeNumber",
            new { SeriesId = seriesId, EpisodeNumber = episodeNumber }).ConfigureAwait(false);
    }

    public async Task<List<WebcomicEpisode>> GetUnreadAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<WebcomicEpisode>(
            "SELECT * FROM \"WebcomicEpisodes\" WHERE \"IsRead\" = 0 ORDER BY \"EpisodeNumber\" DESC",
            new { }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<WebcomicEpisode> GetUnread()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<WebcomicEpisode>(
            "SELECT * FROM \"WebcomicEpisodes\" WHERE \"IsRead\" = 0 ORDER BY \"EpisodeNumber\" DESC",
            new { }).ToList();
    }

    public async Task<List<WebcomicEpisode>> GetUnreadBySeriesAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<WebcomicEpisode>(
            "SELECT * FROM \"WebcomicEpisodes\" WHERE \"WebcomicSeriesId\" = @SeriesId AND \"IsRead\" = 0 ORDER BY \"EpisodeNumber\" ASC",
            new { SeriesId = seriesId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<int> GetUnreadCountAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.ExecuteScalarAsync<int>(
            "SELECT COUNT(*) FROM \"WebcomicEpisodes\" WHERE \"IsRead\" = 0").ConfigureAwait(false);
    }

    public async Task<int> GetUnreadCountBySeriesAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.ExecuteScalarAsync<int>(
            "SELECT COUNT(*) FROM \"WebcomicEpisodes\" WHERE \"IsRead\" = 0 AND \"WebcomicSeriesId\" = @SeriesId",
            new { SeriesId = seriesId }).ConfigureAwait(false);
    }

    public int GetUnreadCount()
    {
        using var conn = _database.OpenConnection();
        return conn.ExecuteScalar<int>(
            "SELECT COUNT(*) FROM \"WebcomicEpisodes\" WHERE \"IsRead\" = 0");
    }

    public async Task MarkReadAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "UPDATE \"WebcomicEpisodes\" SET \"IsRead\" = 1 WHERE \"Id\" = @Id",
            new { Id = id }).ConfigureAwait(false);
    }

    public async Task MarkUnreadAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "UPDATE \"WebcomicEpisodes\" SET \"IsRead\" = 0 WHERE \"Id\" = @Id",
            new { Id = id }).ConfigureAwait(false);
    }

    public async Task MarkAllReadBySeriesAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "UPDATE \"WebcomicEpisodes\" SET \"IsRead\" = 1 WHERE \"WebcomicSeriesId\" = @SeriesId",
            new { SeriesId = seriesId }).ConfigureAwait(false);
    }
}
