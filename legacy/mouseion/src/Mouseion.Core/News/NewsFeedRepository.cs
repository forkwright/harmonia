// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.News;

public interface INewsFeedRepository : IBasicRepository<NewsFeed>
{
    Task<NewsFeed?> FindByFeedUrlAsync(string feedUrl, CancellationToken ct = default);
    Task<List<NewsFeed>> GetMonitoredAsync(CancellationToken ct = default);
    Task<List<NewsFeed>> GetByRootFolderAsync(string rootFolder, CancellationToken ct = default);
    Task<List<NewsFeed>> GetDueForRefreshAsync(CancellationToken ct = default);

    NewsFeed? FindByFeedUrl(string feedUrl);
    List<NewsFeed> GetMonitored();
    List<NewsFeed> GetByRootFolder(string rootFolder);
}

public class NewsFeedRepository : BasicRepository<NewsFeed>, INewsFeedRepository
{
    public NewsFeedRepository(IDatabase database)
        : base(database, "NewsFeeds")
    {
    }

    public async Task<NewsFeed?> FindByFeedUrlAsync(string feedUrl, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<NewsFeed>(
            "SELECT * FROM \"NewsFeeds\" WHERE \"FeedUrl\" = @FeedUrl",
            new { FeedUrl = feedUrl }).ConfigureAwait(false);
    }

    public NewsFeed? FindByFeedUrl(string feedUrl)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<NewsFeed>(
            "SELECT * FROM \"NewsFeeds\" WHERE \"FeedUrl\" = @FeedUrl",
            new { FeedUrl = feedUrl });
    }

    public async Task<List<NewsFeed>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<NewsFeed>(
            "SELECT * FROM \"NewsFeeds\" WHERE \"Monitored\" = 1",
            new { }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<NewsFeed> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<NewsFeed>(
            "SELECT * FROM \"NewsFeeds\" WHERE \"Monitored\" = 1",
            new { }).ToList();
    }

    public async Task<List<NewsFeed>> GetByRootFolderAsync(string rootFolder, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<NewsFeed>(
            "SELECT * FROM \"NewsFeeds\" WHERE \"RootFolderPath\" = @RootFolder",
            new { RootFolder = rootFolder }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<NewsFeed> GetByRootFolder(string rootFolder)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<NewsFeed>(
            "SELECT * FROM \"NewsFeeds\" WHERE \"RootFolderPath\" = @RootFolder",
            new { RootFolder = rootFolder }).ToList();
    }

    public async Task<List<NewsFeed>> GetDueForRefreshAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<NewsFeed>(
            @"SELECT * FROM ""NewsFeeds""
              WHERE ""Monitored"" = 1
              AND (""LastFetchTime"" IS NULL
                   OR datetime(""LastFetchTime"", '+' || ""RefreshInterval"" || ' minutes') < datetime('now'))",
            new { }).ConfigureAwait(false);
        return result.ToList();
    }
}
