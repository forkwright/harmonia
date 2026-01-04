// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Podcasts;

public interface IPodcastFileRepository : IBasicRepository<PodcastFile>
{
    Task<List<PodcastFile>> GetByEpisodeIdAsync(int episodeId, CancellationToken ct = default);
    Task<List<PodcastFile>> GetByShowIdAsync(int showId, CancellationToken ct = default);
    Task<PodcastFile?> FindByPathAsync(string relativePath, CancellationToken ct = default);

    List<PodcastFile> GetByEpisodeId(int episodeId);
    List<PodcastFile> GetByShowId(int showId);
    PodcastFile? FindByPath(string relativePath);
}

public class PodcastFileRepository : BasicRepository<PodcastFile>, IPodcastFileRepository
{
    public PodcastFileRepository(IDatabase database)
        : base(database, "PodcastFiles")
    {
    }

    public async Task<List<PodcastFile>> GetByEpisodeIdAsync(int episodeId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<PodcastFile>(
            "SELECT * FROM \"PodcastFiles\" WHERE \"PodcastEpisodeId\" = @EpisodeId",
            new { EpisodeId = episodeId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<PodcastFile> GetByEpisodeId(int episodeId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<PodcastFile>(
            "SELECT * FROM \"PodcastFiles\" WHERE \"PodcastEpisodeId\" = @EpisodeId",
            new { EpisodeId = episodeId }).ToList();
    }

    public async Task<List<PodcastFile>> GetByShowIdAsync(int showId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<PodcastFile>(
            "SELECT * FROM \"PodcastFiles\" WHERE \"PodcastShowId\" = @ShowId",
            new { ShowId = showId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<PodcastFile> GetByShowId(int showId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<PodcastFile>(
            "SELECT * FROM \"PodcastFiles\" WHERE \"PodcastShowId\" = @ShowId",
            new { ShowId = showId }).ToList();
    }

    public async Task<PodcastFile?> FindByPathAsync(string relativePath, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<PodcastFile>(
            "SELECT * FROM \"PodcastFiles\" WHERE \"RelativePath\" = @RelativePath",
            new { RelativePath = relativePath }).ConfigureAwait(false);
    }

    public PodcastFile? FindByPath(string relativePath)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<PodcastFile>(
            "SELECT * FROM \"PodcastFiles\" WHERE \"RelativePath\" = @RelativePath",
            new { RelativePath = relativePath });
    }
}
