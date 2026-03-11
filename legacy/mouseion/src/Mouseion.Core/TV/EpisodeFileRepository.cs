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

public interface IEpisodeFileRepository : IBasicRepository<EpisodeFile>
{
    Task<List<EpisodeFile>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default);
    Task<List<EpisodeFile>> GetByPathAsync(string path, CancellationToken ct = default);

    List<EpisodeFile> GetBySeriesId(int seriesId);
    List<EpisodeFile> GetByPath(string path);
}

public class EpisodeFileRepository : BasicRepository<EpisodeFile>, IEpisodeFileRepository
{
    public EpisodeFileRepository(IDatabase database)
        : base(database, "EpisodeFiles")
    {
    }

    public async Task<List<EpisodeFile>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<EpisodeFile>(
            "SELECT * FROM \"EpisodeFiles\" WHERE \"SeriesId\" = @SeriesId",
            new { SeriesId = seriesId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<EpisodeFile> GetBySeriesId(int seriesId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<EpisodeFile>(
            "SELECT * FROM \"EpisodeFiles\" WHERE \"SeriesId\" = @SeriesId",
            new { SeriesId = seriesId }).ToList();
    }

    public async Task<List<EpisodeFile>> GetByPathAsync(string path, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<EpisodeFile>(
            "SELECT * FROM \"EpisodeFiles\" WHERE \"RelativePath\" LIKE @Path",
            new { Path = $"{path}%" }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<EpisodeFile> GetByPath(string path)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<EpisodeFile>(
            "SELECT * FROM \"EpisodeFiles\" WHERE \"RelativePath\" LIKE @Path",
            new { Path = $"{path}%" }).ToList();
    }
}
