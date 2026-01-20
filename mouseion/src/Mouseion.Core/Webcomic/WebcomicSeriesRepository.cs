// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Webcomic;

public interface IWebcomicSeriesRepository : IBasicRepository<WebcomicSeries>
{
    Task<WebcomicSeries?> FindByWebtoonIdAsync(string webtoonId, CancellationToken ct = default);
    Task<WebcomicSeries?> FindByTapasIdAsync(string tapasId, CancellationToken ct = default);
    Task<List<WebcomicSeries>> GetMonitoredAsync(CancellationToken ct = default);
    Task<List<WebcomicSeries>> GetByRootFolderAsync(string rootFolder, CancellationToken ct = default);
    Task<List<WebcomicSeries>> GetByPlatformAsync(string platform, CancellationToken ct = default);

    WebcomicSeries? FindByWebtoonId(string webtoonId);
    WebcomicSeries? FindByTapasId(string tapasId);
    List<WebcomicSeries> GetMonitored();
    List<WebcomicSeries> GetByRootFolder(string rootFolder);
}

public class WebcomicSeriesRepository : BasicRepository<WebcomicSeries>, IWebcomicSeriesRepository
{
    public WebcomicSeriesRepository(IDatabase database)
        : base(database, "WebcomicSeries")
    {
    }

    public async Task<WebcomicSeries?> FindByWebtoonIdAsync(string webtoonId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<WebcomicSeries>(
            "SELECT * FROM \"WebcomicSeries\" WHERE \"WebtoonId\" = @WebtoonId",
            new { WebtoonId = webtoonId }).ConfigureAwait(false);
    }

    public WebcomicSeries? FindByWebtoonId(string webtoonId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<WebcomicSeries>(
            "SELECT * FROM \"WebcomicSeries\" WHERE \"WebtoonId\" = @WebtoonId",
            new { WebtoonId = webtoonId });
    }

    public async Task<WebcomicSeries?> FindByTapasIdAsync(string tapasId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<WebcomicSeries>(
            "SELECT * FROM \"WebcomicSeries\" WHERE \"TapasId\" = @TapasId",
            new { TapasId = tapasId }).ConfigureAwait(false);
    }

    public WebcomicSeries? FindByTapasId(string tapasId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<WebcomicSeries>(
            "SELECT * FROM \"WebcomicSeries\" WHERE \"TapasId\" = @TapasId",
            new { TapasId = tapasId });
    }

    public async Task<List<WebcomicSeries>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<WebcomicSeries>(
            "SELECT * FROM \"WebcomicSeries\" WHERE \"Monitored\" = 1",
            new { }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<WebcomicSeries> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<WebcomicSeries>(
            "SELECT * FROM \"WebcomicSeries\" WHERE \"Monitored\" = 1",
            new { }).ToList();
    }

    public async Task<List<WebcomicSeries>> GetByRootFolderAsync(string rootFolder, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<WebcomicSeries>(
            "SELECT * FROM \"WebcomicSeries\" WHERE \"RootFolderPath\" = @RootFolder",
            new { RootFolder = rootFolder }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<WebcomicSeries> GetByRootFolder(string rootFolder)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<WebcomicSeries>(
            "SELECT * FROM \"WebcomicSeries\" WHERE \"RootFolderPath\" = @RootFolder",
            new { RootFolder = rootFolder }).ToList();
    }

    public async Task<List<WebcomicSeries>> GetByPlatformAsync(string platform, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<WebcomicSeries>(
            "SELECT * FROM \"WebcomicSeries\" WHERE \"Platform\" = @Platform",
            new { Platform = platform }).ConfigureAwait(false);
        return result.ToList();
    }
}
