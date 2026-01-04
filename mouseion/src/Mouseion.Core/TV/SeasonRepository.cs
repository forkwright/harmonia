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

public interface ISeasonRepository : IBasicRepository<Season>
{
    Task<List<Season>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default);
    Task<Season?> FindBySeriesAndSeasonAsync(int seriesId, int seasonNumber, CancellationToken ct = default);

    List<Season> GetBySeriesId(int seriesId);
    Season? FindBySeriesAndSeason(int seriesId, int seasonNumber);
}

public class SeasonRepository : BasicRepository<Season>, ISeasonRepository
{
    public SeasonRepository(IDatabase database)
        : base(database, "Seasons")
    {
    }

    public async Task<List<Season>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Season>(
            "SELECT * FROM \"Seasons\" WHERE \"SeriesId\" = @SeriesId ORDER BY \"SeasonNumber\"",
            new { SeriesId = seriesId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Season> GetBySeriesId(int seriesId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Season>(
            "SELECT * FROM \"Seasons\" WHERE \"SeriesId\" = @SeriesId ORDER BY \"SeasonNumber\"",
            new { SeriesId = seriesId }).ToList();
    }

    public async Task<Season?> FindBySeriesAndSeasonAsync(int seriesId, int seasonNumber, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Season>(
            "SELECT * FROM \"Seasons\" WHERE \"SeriesId\" = @SeriesId AND \"SeasonNumber\" = @SeasonNumber",
            new { SeriesId = seriesId, SeasonNumber = seasonNumber }).ConfigureAwait(false);
    }

    public Season? FindBySeriesAndSeason(int seriesId, int seasonNumber)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Season>(
            "SELECT * FROM \"Seasons\" WHERE \"SeriesId\" = @SeriesId AND \"SeasonNumber\" = @SeasonNumber",
            new { SeriesId = seriesId, SeasonNumber = seasonNumber });
    }
}
