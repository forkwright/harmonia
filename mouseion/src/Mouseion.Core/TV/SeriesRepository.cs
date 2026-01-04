// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.TV;

public interface ISeriesRepository : IBasicRepository<Series>
{
    Task<Series?> FindByTvdbIdAsync(int tvdbId, CancellationToken ct = default);
    Task<Series?> FindByTitleAsync(string cleanTitle, CancellationToken ct = default);
    Task<List<Series>> GetMonitoredAsync(CancellationToken ct = default);

    Series? FindByTvdbId(int tvdbId);
    Series? FindByTitle(string cleanTitle);
    List<Series> GetMonitored();
}

public class SeriesRepository : BasicRepository<Series>, ISeriesRepository
{
    public SeriesRepository(IDatabase database)
        : base(database, "Series")
    {
    }

    public async Task<Series?> FindByTvdbIdAsync(int tvdbId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Series>(
            "SELECT * FROM \"Series\" WHERE \"TvdbId\" = @TvdbId",
            new { TvdbId = tvdbId }).ConfigureAwait(false);
    }

    public Series? FindByTvdbId(int tvdbId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Series>(
            "SELECT * FROM \"Series\" WHERE \"TvdbId\" = @TvdbId",
            new { TvdbId = tvdbId });
    }

    public async Task<Series?> FindByTitleAsync(string cleanTitle, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Series>(
            "SELECT * FROM \"Series\" WHERE \"CleanTitle\" = @CleanTitle",
            new { CleanTitle = cleanTitle }).ConfigureAwait(false);
    }

    public Series? FindByTitle(string cleanTitle)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Series>(
            "SELECT * FROM \"Series\" WHERE \"CleanTitle\" = @CleanTitle",
            new { CleanTitle = cleanTitle });
    }

    public async Task<List<Series>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Series>(
            "SELECT * FROM \"Series\" WHERE \"Monitored\" = @Monitored",
            new { Monitored = true }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Series> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Series>(
            "SELECT * FROM \"Series\" WHERE \"Monitored\" = @Monitored",
            new { Monitored = true }).ToList();
    }
}
