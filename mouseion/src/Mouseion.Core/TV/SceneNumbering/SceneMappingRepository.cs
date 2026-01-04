// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.TV.SceneNumbering;

public interface ISceneMappingRepository : IBasicRepository<SceneMapping>
{
    Task<List<SceneMapping>> GetByTvdbIdAsync(int tvdbId, CancellationToken ct = default);
    Task<SceneMapping?> FindMappingAsync(int tvdbId, int seasonNumber, int episodeNumber, CancellationToken ct = default);

    List<SceneMapping> GetByTvdbId(int tvdbId);
    SceneMapping? FindMapping(int tvdbId, int seasonNumber, int episodeNumber);
}

public class SceneMappingRepository : BasicRepository<SceneMapping>, ISceneMappingRepository
{
    public SceneMappingRepository(IDatabase database)
        : base(database, "SceneMappings")
    {
    }

    public async Task<List<SceneMapping>> GetByTvdbIdAsync(int tvdbId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<SceneMapping>(
            "SELECT * FROM \"SceneMappings\" WHERE \"TvdbId\" = @TvdbId",
            new { TvdbId = tvdbId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<SceneMapping> GetByTvdbId(int tvdbId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<SceneMapping>(
            "SELECT * FROM \"SceneMappings\" WHERE \"TvdbId\" = @TvdbId",
            new { TvdbId = tvdbId }).ToList();
    }

    public async Task<SceneMapping?> FindMappingAsync(int tvdbId, int seasonNumber, int episodeNumber, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<SceneMapping>(
            "SELECT * FROM \"SceneMappings\" WHERE \"TvdbId\" = @TvdbId AND \"SeasonNumber\" = @SeasonNumber AND \"EpisodeNumber\" = @EpisodeNumber",
            new { TvdbId = tvdbId, SeasonNumber = seasonNumber, EpisodeNumber = episodeNumber }).ConfigureAwait(false);
    }

    public SceneMapping? FindMapping(int tvdbId, int seasonNumber, int episodeNumber)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<SceneMapping>(
            "SELECT * FROM \"SceneMappings\" WHERE \"TvdbId\" = @TvdbId AND \"SeasonNumber\" = @SeasonNumber AND \"EpisodeNumber\" = @EpisodeNumber",
            new { TvdbId = tvdbId, SeasonNumber = seasonNumber, EpisodeNumber = episodeNumber });
    }
}
