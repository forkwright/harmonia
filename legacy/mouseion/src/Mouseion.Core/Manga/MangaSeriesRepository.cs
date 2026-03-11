// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Manga;

public interface IMangaSeriesRepository : IBasicRepository<MangaSeries>
{
    Task<MangaSeries?> FindByMangaDexIdAsync(string mangaDexId, CancellationToken ct = default);
    Task<MangaSeries?> FindByAniListIdAsync(int aniListId, CancellationToken ct = default);
    Task<List<MangaSeries>> GetMonitoredAsync(CancellationToken ct = default);
    Task<List<MangaSeries>> GetByRootFolderAsync(string rootFolder, CancellationToken ct = default);

    MangaSeries? FindByMangaDexId(string mangaDexId);
    MangaSeries? FindByAniListId(int aniListId);
    List<MangaSeries> GetMonitored();
    List<MangaSeries> GetByRootFolder(string rootFolder);
}

public class MangaSeriesRepository : BasicRepository<MangaSeries>, IMangaSeriesRepository
{
    public MangaSeriesRepository(IDatabase database)
        : base(database, "MangaSeries")
    {
    }

    public async Task<MangaSeries?> FindByMangaDexIdAsync(string mangaDexId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<MangaSeries>(
            "SELECT * FROM \"MangaSeries\" WHERE \"MangaDexId\" = @MangaDexId",
            new { MangaDexId = mangaDexId }).ConfigureAwait(false);
    }

    public MangaSeries? FindByMangaDexId(string mangaDexId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<MangaSeries>(
            "SELECT * FROM \"MangaSeries\" WHERE \"MangaDexId\" = @MangaDexId",
            new { MangaDexId = mangaDexId });
    }

    public async Task<MangaSeries?> FindByAniListIdAsync(int aniListId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<MangaSeries>(
            "SELECT * FROM \"MangaSeries\" WHERE \"AniListId\" = @AniListId",
            new { AniListId = aniListId }).ConfigureAwait(false);
    }

    public MangaSeries? FindByAniListId(int aniListId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<MangaSeries>(
            "SELECT * FROM \"MangaSeries\" WHERE \"AniListId\" = @AniListId",
            new { AniListId = aniListId });
    }

    public async Task<List<MangaSeries>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<MangaSeries>(
            "SELECT * FROM \"MangaSeries\" WHERE \"Monitored\" = 1",
            new { }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<MangaSeries> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<MangaSeries>(
            "SELECT * FROM \"MangaSeries\" WHERE \"Monitored\" = 1",
            new { }).ToList();
    }

    public async Task<List<MangaSeries>> GetByRootFolderAsync(string rootFolder, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<MangaSeries>(
            "SELECT * FROM \"MangaSeries\" WHERE \"RootFolderPath\" = @RootFolder",
            new { RootFolder = rootFolder }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<MangaSeries> GetByRootFolder(string rootFolder)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<MangaSeries>(
            "SELECT * FROM \"MangaSeries\" WHERE \"RootFolderPath\" = @RootFolder",
            new { RootFolder = rootFolder }).ToList();
    }
}
