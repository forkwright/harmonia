// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.News;

public interface INewsArticleRepository : IBasicRepository<NewsArticle>
{
    Task<List<NewsArticle>> GetByFeedIdAsync(int feedId, CancellationToken ct = default);
    Task<NewsArticle?> FindByGuidAsync(string guid, CancellationToken ct = default);
    Task<NewsArticle?> FindByGuidAndFeedAsync(string guid, int feedId, CancellationToken ct = default);
    Task<List<NewsArticle>> GetUnreadAsync(CancellationToken ct = default);
    Task<List<NewsArticle>> GetStarredAsync(CancellationToken ct = default);
    Task<List<NewsArticle>> GetRecentAsync(int count, CancellationToken ct = default);
    Task<int> GetUnreadCountAsync(CancellationToken ct = default);
    Task<int> GetUnreadCountByFeedAsync(int feedId, CancellationToken ct = default);
    Task MarkReadAsync(int id, CancellationToken ct = default);
    Task MarkUnreadAsync(int id, CancellationToken ct = default);
    Task MarkAllReadByFeedAsync(int feedId, CancellationToken ct = default);
    Task SetStarredAsync(int id, bool starred, CancellationToken ct = default);

    List<NewsArticle> GetByFeedId(int feedId);
    NewsArticle? FindByGuid(string guid);
    List<NewsArticle> GetUnread();
    List<NewsArticle> GetStarred();
    int GetUnreadCount();
}

public class NewsArticleRepository : BasicRepository<NewsArticle>, INewsArticleRepository
{
    public NewsArticleRepository(IDatabase database)
        : base(database, "NewsArticles")
    {
    }

    public async Task<List<NewsArticle>> GetByFeedIdAsync(int feedId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<NewsArticle>(
            "SELECT * FROM \"NewsArticles\" WHERE \"NewsFeedId\" = @FeedId ORDER BY \"PublishDate\" DESC",
            new { FeedId = feedId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<NewsArticle> GetByFeedId(int feedId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<NewsArticle>(
            "SELECT * FROM \"NewsArticles\" WHERE \"NewsFeedId\" = @FeedId ORDER BY \"PublishDate\" DESC",
            new { FeedId = feedId }).ToList();
    }

    public async Task<NewsArticle?> FindByGuidAsync(string guid, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<NewsArticle>(
            "SELECT * FROM \"NewsArticles\" WHERE \"ArticleGuid\" = @Guid",
            new { Guid = guid }).ConfigureAwait(false);
    }

    public async Task<NewsArticle?> FindByGuidAndFeedAsync(string guid, int feedId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<NewsArticle>(
            "SELECT * FROM \"NewsArticles\" WHERE \"ArticleGuid\" = @Guid AND \"NewsFeedId\" = @FeedId",
            new { Guid = guid, FeedId = feedId }).ConfigureAwait(false);
    }

    public NewsArticle? FindByGuid(string guid)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<NewsArticle>(
            "SELECT * FROM \"NewsArticles\" WHERE \"ArticleGuid\" = @Guid",
            new { Guid = guid });
    }

    public async Task<List<NewsArticle>> GetUnreadAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<NewsArticle>(
            "SELECT * FROM \"NewsArticles\" WHERE \"IsRead\" = 0 ORDER BY \"PublishDate\" DESC",
            new { }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<NewsArticle> GetUnread()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<NewsArticle>(
            "SELECT * FROM \"NewsArticles\" WHERE \"IsRead\" = 0 ORDER BY \"PublishDate\" DESC",
            new { }).ToList();
    }

    public async Task<List<NewsArticle>> GetStarredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<NewsArticle>(
            "SELECT * FROM \"NewsArticles\" WHERE \"IsStarred\" = 1 ORDER BY \"PublishDate\" DESC",
            new { }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<NewsArticle> GetStarred()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<NewsArticle>(
            "SELECT * FROM \"NewsArticles\" WHERE \"IsStarred\" = 1 ORDER BY \"PublishDate\" DESC",
            new { }).ToList();
    }

    public async Task<List<NewsArticle>> GetRecentAsync(int count, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<NewsArticle>(
            "SELECT * FROM \"NewsArticles\" ORDER BY \"PublishDate\" DESC LIMIT @Count",
            new { Count = count }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<int> GetUnreadCountAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.ExecuteScalarAsync<int>(
            "SELECT COUNT(*) FROM \"NewsArticles\" WHERE \"IsRead\" = 0").ConfigureAwait(false);
    }

    public async Task<int> GetUnreadCountByFeedAsync(int feedId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.ExecuteScalarAsync<int>(
            "SELECT COUNT(*) FROM \"NewsArticles\" WHERE \"IsRead\" = 0 AND \"NewsFeedId\" = @FeedId",
            new { FeedId = feedId }).ConfigureAwait(false);
    }

    public int GetUnreadCount()
    {
        using var conn = _database.OpenConnection();
        return conn.ExecuteScalar<int>(
            "SELECT COUNT(*) FROM \"NewsArticles\" WHERE \"IsRead\" = 0");
    }

    public async Task MarkReadAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "UPDATE \"NewsArticles\" SET \"IsRead\" = 1 WHERE \"Id\" = @Id",
            new { Id = id }).ConfigureAwait(false);
    }

    public async Task MarkUnreadAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "UPDATE \"NewsArticles\" SET \"IsRead\" = 0 WHERE \"Id\" = @Id",
            new { Id = id }).ConfigureAwait(false);
    }

    public async Task MarkAllReadByFeedAsync(int feedId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "UPDATE \"NewsArticles\" SET \"IsRead\" = 1 WHERE \"NewsFeedId\" = @FeedId",
            new { FeedId = feedId }).ConfigureAwait(false);
    }

    public async Task SetStarredAsync(int id, bool starred, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "UPDATE \"NewsArticles\" SET \"IsStarred\" = @Starred WHERE \"Id\" = @Id",
            new { Id = id, Starred = starred ? 1 : 0 }).ConfigureAwait(false);
    }
}
