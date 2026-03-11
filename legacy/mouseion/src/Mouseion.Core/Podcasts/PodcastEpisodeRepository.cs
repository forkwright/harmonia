// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Podcasts;

public interface IPodcastEpisodeRepository : IBasicRepository<PodcastEpisode>
{
    Task<List<PodcastEpisode>> GetByShowIdAsync(int showId, CancellationToken ct = default);
    Task<PodcastEpisode?> FindByGuidAsync(string guid, CancellationToken ct = default);
    Task<List<PodcastEpisode>> GetMonitoredAsync(int showId, CancellationToken ct = default);
    Task<List<PodcastEpisode>> GetRecentEpisodesAsync(int count, CancellationToken ct = default);

    List<PodcastEpisode> GetByShowId(int showId);
    PodcastEpisode? FindByGuid(string guid);
    List<PodcastEpisode> GetMonitored(int showId);
    List<PodcastEpisode> GetRecentEpisodes(int count);
}

public class PodcastEpisodeRepository : BasicRepository<PodcastEpisode>, IPodcastEpisodeRepository
{
    public PodcastEpisodeRepository(IDatabase database)
        : base(database, "PodcastEpisodes")
    {
    }

    public async Task<List<PodcastEpisode>> GetByShowIdAsync(int showId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<PodcastEpisode>(
            "SELECT * FROM \"PodcastEpisodes\" WHERE \"PodcastShowId\" = @ShowId ORDER BY \"PublishDate\" DESC",
            new { ShowId = showId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<PodcastEpisode> GetByShowId(int showId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<PodcastEpisode>(
            "SELECT * FROM \"PodcastEpisodes\" WHERE \"PodcastShowId\" = @ShowId ORDER BY \"PublishDate\" DESC",
            new { ShowId = showId }).ToList();
    }

    public async Task<PodcastEpisode?> FindByGuidAsync(string guid, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<PodcastEpisode>(
            "SELECT * FROM \"PodcastEpisodes\" WHERE \"EpisodeGuid\" = @Guid",
            new { Guid = guid }).ConfigureAwait(false);
    }

    public PodcastEpisode? FindByGuid(string guid)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<PodcastEpisode>(
            "SELECT * FROM \"PodcastEpisodes\" WHERE \"EpisodeGuid\" = @Guid",
            new { Guid = guid });
    }

    public async Task<List<PodcastEpisode>> GetMonitoredAsync(int showId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<PodcastEpisode>(
            "SELECT * FROM \"PodcastEpisodes\" WHERE \"PodcastShowId\" = @ShowId AND \"Monitored\" = 1 ORDER BY \"PublishDate\" DESC",
            new { ShowId = showId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<PodcastEpisode> GetMonitored(int showId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<PodcastEpisode>(
            "SELECT * FROM \"PodcastEpisodes\" WHERE \"PodcastShowId\" = @ShowId AND \"Monitored\" = 1 ORDER BY \"PublishDate\" DESC",
            new { ShowId = showId }).ToList();
    }

    public async Task<List<PodcastEpisode>> GetRecentEpisodesAsync(int count, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<PodcastEpisode>(
            "SELECT * FROM \"PodcastEpisodes\" ORDER BY \"PublishDate\" DESC LIMIT @Count",
            new { Count = count }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<PodcastEpisode> GetRecentEpisodes(int count)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<PodcastEpisode>(
            "SELECT * FROM \"PodcastEpisodes\" ORDER BY \"PublishDate\" DESC LIMIT @Count",
            new { Count = count }).ToList();
    }
}
