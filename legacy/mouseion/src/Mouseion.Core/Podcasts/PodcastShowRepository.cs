// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Podcasts;

public interface IPodcastShowRepository : IBasicRepository<PodcastShow>
{
    Task<PodcastShow?> FindByFeedUrlAsync(string feedUrl, CancellationToken ct = default);
    Task<PodcastShow?> FindByForeignIdAsync(string foreignId, CancellationToken ct = default);
    Task<List<PodcastShow>> GetMonitoredAsync(CancellationToken ct = default);
    Task<List<PodcastShow>> GetByRootFolderAsync(string rootFolder, CancellationToken ct = default);

    PodcastShow? FindByFeedUrl(string feedUrl);
    PodcastShow? FindByForeignId(string foreignId);
    List<PodcastShow> GetMonitored();
    List<PodcastShow> GetByRootFolder(string rootFolder);
}

public class PodcastShowRepository : BasicRepository<PodcastShow>, IPodcastShowRepository
{
    public PodcastShowRepository(IDatabase database)
        : base(database, "PodcastShows")
    {
    }

    public async Task<PodcastShow?> FindByFeedUrlAsync(string feedUrl, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<PodcastShow>(
            "SELECT * FROM \"PodcastShows\" WHERE \"FeedUrl\" = @FeedUrl",
            new { FeedUrl = feedUrl }).ConfigureAwait(false);
    }

    public PodcastShow? FindByFeedUrl(string feedUrl)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<PodcastShow>(
            "SELECT * FROM \"PodcastShows\" WHERE \"FeedUrl\" = @FeedUrl",
            new { FeedUrl = feedUrl });
    }

    public async Task<PodcastShow?> FindByForeignIdAsync(string foreignId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<PodcastShow>(
            "SELECT * FROM \"PodcastShows\" WHERE \"ForeignPodcastId\" = @ForeignId",
            new { ForeignId = foreignId }).ConfigureAwait(false);
    }

    public PodcastShow? FindByForeignId(string foreignId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<PodcastShow>(
            "SELECT * FROM \"PodcastShows\" WHERE \"ForeignPodcastId\" = @ForeignId",
            new { ForeignId = foreignId });
    }

    public async Task<List<PodcastShow>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<PodcastShow>(
            "SELECT * FROM \"PodcastShows\" WHERE \"Monitored\" = 1",
            new { }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<PodcastShow> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<PodcastShow>(
            "SELECT * FROM \"PodcastShows\" WHERE \"Monitored\" = 1",
            new { }).ToList();
    }

    public async Task<List<PodcastShow>> GetByRootFolderAsync(string rootFolder, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<PodcastShow>(
            "SELECT * FROM \"PodcastShows\" WHERE \"RootFolderPath\" = @RootFolder",
            new { RootFolder = rootFolder }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<PodcastShow> GetByRootFolder(string rootFolder)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<PodcastShow>(
            "SELECT * FROM \"PodcastShows\" WHERE \"RootFolderPath\" = @RootFolder",
            new { RootFolder = rootFolder }).ToList();
    }
}
