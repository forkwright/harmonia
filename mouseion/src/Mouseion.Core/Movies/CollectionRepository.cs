// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Movies;

public interface ICollectionRepository : IBasicRepository<Collection>
{
    Task<Collection?> FindByTmdbIdAsync(string tmdbId, CancellationToken ct = default);
    Task<List<Collection>> GetMonitoredAsync(CancellationToken ct = default);

    Collection? FindByTmdbId(string tmdbId);
    List<Collection> GetMonitored();
}

public class CollectionRepository : BasicRepository<Collection>, ICollectionRepository
{
    public CollectionRepository(IDatabase database)
        : base(database, "Collections")
    {
    }

    public async Task<Collection?> FindByTmdbIdAsync(string tmdbId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Collection>(
            "SELECT * FROM \"Collections\" WHERE \"TmdbId\" = @TmdbId",
            new { TmdbId = tmdbId }).ConfigureAwait(false);
    }

    public Collection? FindByTmdbId(string tmdbId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Collection>(
            "SELECT * FROM \"Collections\" WHERE \"TmdbId\" = @TmdbId",
            new { TmdbId = tmdbId });
    }

    public async Task<List<Collection>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Collection>(
            "SELECT * FROM \"Collections\" WHERE \"Monitored\" = @Monitored",
            new { Monitored = true }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Collection> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Collection>(
            "SELECT * FROM \"Collections\" WHERE \"Monitored\" = @Monitored",
            new { Monitored = true }).ToList();
    }
}
