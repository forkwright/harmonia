// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Comic;

public interface IComicIssueRepository : IBasicRepository<ComicIssue>
{
    Task<List<ComicIssue>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default);
    Task<ComicIssue?> FindByComicVineIssueIdAsync(int comicVineIssueId, CancellationToken ct = default);
    Task<ComicIssue?> FindByIssueNumberAsync(int seriesId, string issueNumber, CancellationToken ct = default);
    Task<List<ComicIssue>> GetUnreadAsync(CancellationToken ct = default);
    Task<List<ComicIssue>> GetUnreadBySeriesAsync(int seriesId, CancellationToken ct = default);
    Task<int> GetUnreadCountAsync(CancellationToken ct = default);
    Task<int> GetUnreadCountBySeriesAsync(int seriesId, CancellationToken ct = default);
    Task MarkReadAsync(int id, CancellationToken ct = default);
    Task MarkUnreadAsync(int id, CancellationToken ct = default);
    Task MarkAllReadBySeriesAsync(int seriesId, CancellationToken ct = default);

    List<ComicIssue> GetBySeriesId(int seriesId);
    ComicIssue? FindByComicVineIssueId(int comicVineIssueId);
    List<ComicIssue> GetUnread();
    int GetUnreadCount();
}

public class ComicIssueRepository : BasicRepository<ComicIssue>, IComicIssueRepository
{
    public ComicIssueRepository(IDatabase database)
        : base(database, "ComicIssues")
    {
    }

    public async Task<List<ComicIssue>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<ComicIssue>(
            "SELECT * FROM \"ComicIssues\" WHERE \"ComicSeriesId\" = @SeriesId ORDER BY \"IssueNumber\" DESC",
            new { SeriesId = seriesId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<ComicIssue> GetBySeriesId(int seriesId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<ComicIssue>(
            "SELECT * FROM \"ComicIssues\" WHERE \"ComicSeriesId\" = @SeriesId ORDER BY \"IssueNumber\" DESC",
            new { SeriesId = seriesId }).ToList();
    }

    public async Task<ComicIssue?> FindByComicVineIssueIdAsync(int comicVineIssueId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<ComicIssue>(
            "SELECT * FROM \"ComicIssues\" WHERE \"ComicVineIssueId\" = @ComicVineIssueId",
            new { ComicVineIssueId = comicVineIssueId }).ConfigureAwait(false);
    }

    public ComicIssue? FindByComicVineIssueId(int comicVineIssueId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<ComicIssue>(
            "SELECT * FROM \"ComicIssues\" WHERE \"ComicVineIssueId\" = @ComicVineIssueId",
            new { ComicVineIssueId = comicVineIssueId });
    }

    public async Task<ComicIssue?> FindByIssueNumberAsync(int seriesId, string issueNumber, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<ComicIssue>(
            "SELECT * FROM \"ComicIssues\" WHERE \"ComicSeriesId\" = @SeriesId AND \"IssueNumber\" = @IssueNumber",
            new { SeriesId = seriesId, IssueNumber = issueNumber }).ConfigureAwait(false);
    }

    public async Task<List<ComicIssue>> GetUnreadAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<ComicIssue>(
            "SELECT * FROM \"ComicIssues\" WHERE \"IsRead\" = 0 ORDER BY \"IssueNumber\" DESC",
            new { }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<ComicIssue> GetUnread()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<ComicIssue>(
            "SELECT * FROM \"ComicIssues\" WHERE \"IsRead\" = 0 ORDER BY \"IssueNumber\" DESC",
            new { }).ToList();
    }

    public async Task<List<ComicIssue>> GetUnreadBySeriesAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<ComicIssue>(
            "SELECT * FROM \"ComicIssues\" WHERE \"ComicSeriesId\" = @SeriesId AND \"IsRead\" = 0 ORDER BY \"IssueNumber\" ASC",
            new { SeriesId = seriesId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<int> GetUnreadCountAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.ExecuteScalarAsync<int>(
            "SELECT COUNT(*) FROM \"ComicIssues\" WHERE \"IsRead\" = 0").ConfigureAwait(false);
    }

    public async Task<int> GetUnreadCountBySeriesAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.ExecuteScalarAsync<int>(
            "SELECT COUNT(*) FROM \"ComicIssues\" WHERE \"IsRead\" = 0 AND \"ComicSeriesId\" = @SeriesId",
            new { SeriesId = seriesId }).ConfigureAwait(false);
    }

    public int GetUnreadCount()
    {
        using var conn = _database.OpenConnection();
        return conn.ExecuteScalar<int>(
            "SELECT COUNT(*) FROM \"ComicIssues\" WHERE \"IsRead\" = 0");
    }

    public async Task MarkReadAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "UPDATE \"ComicIssues\" SET \"IsRead\" = 1 WHERE \"Id\" = @Id",
            new { Id = id }).ConfigureAwait(false);
    }

    public async Task MarkUnreadAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "UPDATE \"ComicIssues\" SET \"IsRead\" = 0 WHERE \"Id\" = @Id",
            new { Id = id }).ConfigureAwait(false);
    }

    public async Task MarkAllReadBySeriesAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "UPDATE \"ComicIssues\" SET \"IsRead\" = 1 WHERE \"ComicSeriesId\" = @SeriesId",
            new { SeriesId = seriesId }).ConfigureAwait(false);
    }
}
