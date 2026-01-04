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

public interface IEpisodeRepository : IBasicRepository<Episode>
{
    Task<List<Episode>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default);
    Task<List<Episode>> GetBySeasonAsync(int seriesId, int seasonNumber, CancellationToken ct = default);
    Task<Episode?> FindByEpisodeAsync(int seriesId, int seasonNumber, int episodeNumber, CancellationToken ct = default);
    Task<List<Episode>> GetMonitoredAsync(CancellationToken ct = default);

    List<Episode> GetBySeriesId(int seriesId);
    List<Episode> GetBySeason(int seriesId, int seasonNumber);
    Episode? FindByEpisode(int seriesId, int seasonNumber, int episodeNumber);
    List<Episode> GetMonitored();
}

public class EpisodeRepository : BasicRepository<Episode>, IEpisodeRepository
{
    public EpisodeRepository(IDatabase database)
        : base(database, "Episodes")
    {
    }

    public async Task<List<Episode>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Episode>(
            "SELECT * FROM \"Episodes\" WHERE \"SeriesId\" = @SeriesId ORDER BY \"SeasonNumber\", \"EpisodeNumber\"",
            new { SeriesId = seriesId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Episode> GetBySeriesId(int seriesId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Episode>(
            "SELECT * FROM \"Episodes\" WHERE \"SeriesId\" = @SeriesId ORDER BY \"SeasonNumber\", \"EpisodeNumber\"",
            new { SeriesId = seriesId }).ToList();
    }

    public async Task<List<Episode>> GetBySeasonAsync(int seriesId, int seasonNumber, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Episode>(
            "SELECT * FROM \"Episodes\" WHERE \"SeriesId\" = @SeriesId AND \"SeasonNumber\" = @SeasonNumber ORDER BY \"EpisodeNumber\"",
            new { SeriesId = seriesId, SeasonNumber = seasonNumber }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Episode> GetBySeason(int seriesId, int seasonNumber)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Episode>(
            "SELECT * FROM \"Episodes\" WHERE \"SeriesId\" = @SeriesId AND \"SeasonNumber\" = @SeasonNumber ORDER BY \"EpisodeNumber\"",
            new { SeriesId = seriesId, SeasonNumber = seasonNumber }).ToList();
    }

    public async Task<Episode?> FindByEpisodeAsync(int seriesId, int seasonNumber, int episodeNumber, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Episode>(
            "SELECT * FROM \"Episodes\" WHERE \"SeriesId\" = @SeriesId AND \"SeasonNumber\" = @SeasonNumber AND \"EpisodeNumber\" = @EpisodeNumber",
            new { SeriesId = seriesId, SeasonNumber = seasonNumber, EpisodeNumber = episodeNumber }).ConfigureAwait(false);
    }

    public Episode? FindByEpisode(int seriesId, int seasonNumber, int episodeNumber)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Episode>(
            "SELECT * FROM \"Episodes\" WHERE \"SeriesId\" = @SeriesId AND \"SeasonNumber\" = @SeasonNumber AND \"EpisodeNumber\" = @EpisodeNumber",
            new { SeriesId = seriesId, SeasonNumber = seasonNumber, EpisodeNumber = episodeNumber });
    }

    public async Task<List<Episode>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Episode>(
            "SELECT * FROM \"Episodes\" WHERE \"Monitored\" = @Monitored",
            new { Monitored = true }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Episode> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Episode>(
            "SELECT * FROM \"Episodes\" WHERE \"Monitored\" = @Monitored",
            new { Monitored = true }).ToList();
    }
}
