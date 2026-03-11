// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Comic;

public interface IComicSeriesRepository : IBasicRepository<ComicSeries>
{
    Task<ComicSeries?> FindByComicVineIdAsync(int comicVineId, CancellationToken ct = default);
    Task<List<ComicSeries>> GetMonitoredAsync(CancellationToken ct = default);
    Task<List<ComicSeries>> GetByPublisherAsync(string publisher, CancellationToken ct = default);
    Task<List<ComicSeries>> GetByRootFolderAsync(string rootFolder, CancellationToken ct = default);

    ComicSeries? FindByComicVineId(int comicVineId);
    List<ComicSeries> GetMonitored();
    List<ComicSeries> GetByPublisher(string publisher);
}

public class ComicSeriesRepository : BasicRepository<ComicSeries>, IComicSeriesRepository
{
    public ComicSeriesRepository(IDatabase database)
        : base(database, "ComicSeries")
    {
    }

    public async Task<ComicSeries?> FindByComicVineIdAsync(int comicVineId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<ComicSeries>(
            "SELECT * FROM \"ComicSeries\" WHERE \"ComicVineId\" = @ComicVineId",
            new { ComicVineId = comicVineId }).ConfigureAwait(false);
    }

    public ComicSeries? FindByComicVineId(int comicVineId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<ComicSeries>(
            "SELECT * FROM \"ComicSeries\" WHERE \"ComicVineId\" = @ComicVineId",
            new { ComicVineId = comicVineId });
    }

    public async Task<List<ComicSeries>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<ComicSeries>(
            "SELECT * FROM \"ComicSeries\" WHERE \"Monitored\" = 1",
            new { }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<ComicSeries> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<ComicSeries>(
            "SELECT * FROM \"ComicSeries\" WHERE \"Monitored\" = 1",
            new { }).ToList();
    }

    public async Task<List<ComicSeries>> GetByPublisherAsync(string publisher, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<ComicSeries>(
            "SELECT * FROM \"ComicSeries\" WHERE \"Publisher\" = @Publisher",
            new { Publisher = publisher }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<ComicSeries> GetByPublisher(string publisher)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<ComicSeries>(
            "SELECT * FROM \"ComicSeries\" WHERE \"Publisher\" = @Publisher",
            new { Publisher = publisher }).ToList();
    }

    public async Task<List<ComicSeries>> GetByRootFolderAsync(string rootFolder, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<ComicSeries>(
            "SELECT * FROM \"ComicSeries\" WHERE \"RootFolderPath\" = @RootFolder",
            new { RootFolder = rootFolder }).ConfigureAwait(false);
        return result.ToList();
    }
}
