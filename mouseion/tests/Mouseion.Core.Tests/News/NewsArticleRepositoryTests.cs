// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.News;
using Mouseion.Core.Tests.Repositories;

namespace Mouseion.Core.Tests.News;

public class NewsArticleRepositoryTests : RepositoryTestBase
{
    private readonly NewsFeedRepository _feedRepository;
    private readonly NewsArticleRepository _articleRepository;

    public NewsArticleRepositoryTests()
    {
        _feedRepository = new NewsFeedRepository(Database);
        _articleRepository = new NewsArticleRepository(Database);
    }

    [Fact]
    public async Task InsertAsync_InsertsNewsArticle()
    {
        var feed = await CreateAndInsertFeedAsync();
        var article = CreateNewsArticle(feed.Id, "Test Article", "guid-123");

        var result = await _articleRepository.InsertAsync(article);

        Assert.True(result.Id > 0);
        Assert.Equal("Test Article", result.Title);
    }

    [Fact]
    public async Task FindAsync_ReturnsArticleById()
    {
        var feed = await CreateAndInsertFeedAsync();
        var article = CreateNewsArticle(feed.Id, "Test Article");
        var inserted = await _articleRepository.InsertAsync(article);

        var result = await _articleRepository.FindAsync(inserted.Id);

        Assert.NotNull(result);
        Assert.Equal(inserted.Id, result.Id);
    }

    [Fact]
    public async Task GetByFeedIdAsync_ReturnsArticlesForFeed()
    {
        var feed1 = await CreateAndInsertFeedAsync("https://feed1.com/rss");
        var feed2 = await CreateAndInsertFeedAsync("https://feed2.com/rss");

        await _articleRepository.InsertAsync(CreateNewsArticle(feed1.Id, "Article 1", "guid-1"));
        await _articleRepository.InsertAsync(CreateNewsArticle(feed1.Id, "Article 2", "guid-2"));
        await _articleRepository.InsertAsync(CreateNewsArticle(feed2.Id, "Article 3", "guid-3"));

        var result = await _articleRepository.GetByFeedIdAsync(feed1.Id);

        Assert.Equal(2, result.Count);
        Assert.All(result, a => Assert.Equal(feed1.Id, a.NewsFeedId));
    }

    [Fact]
    public async Task FindByGuidAsync_ReturnsMatchingArticle()
    {
        var feed = await CreateAndInsertFeedAsync();
        await _articleRepository.InsertAsync(CreateNewsArticle(feed.Id, "Test", "unique-guid-123"));

        var result = await _articleRepository.FindByGuidAsync("unique-guid-123");

        Assert.NotNull(result);
        Assert.Equal("unique-guid-123", result.ArticleGuid);
    }

    [Fact]
    public async Task GetUnreadAsync_ReturnsOnlyUnreadArticles()
    {
        var feed = await CreateAndInsertFeedAsync();

        var unread = CreateNewsArticle(feed.Id, "Unread", "guid-1");
        unread.IsRead = false;
        await _articleRepository.InsertAsync(unread);

        var read = CreateNewsArticle(feed.Id, "Read", "guid-2");
        read.IsRead = true;
        await _articleRepository.InsertAsync(read);

        var result = await _articleRepository.GetUnreadAsync();

        Assert.Single(result);
        Assert.Equal("Unread", result[0].Title);
    }

    [Fact]
    public async Task GetStarredAsync_ReturnsOnlyStarredArticles()
    {
        var feed = await CreateAndInsertFeedAsync();

        var starred = CreateNewsArticle(feed.Id, "Starred", "guid-1");
        starred.IsStarred = true;
        await _articleRepository.InsertAsync(starred);

        var notStarred = CreateNewsArticle(feed.Id, "Not Starred", "guid-2");
        notStarred.IsStarred = false;
        await _articleRepository.InsertAsync(notStarred);

        var result = await _articleRepository.GetStarredAsync();

        Assert.Single(result);
        Assert.Equal("Starred", result[0].Title);
    }

    [Fact]
    public async Task MarkReadAsync_SetsIsReadToTrue()
    {
        var feed = await CreateAndInsertFeedAsync();
        var article = CreateNewsArticle(feed.Id, "Test");
        article.IsRead = false;
        var inserted = await _articleRepository.InsertAsync(article);

        await _articleRepository.MarkReadAsync(inserted.Id);
        var result = await _articleRepository.FindAsync(inserted.Id);

        Assert.True(result!.IsRead);
    }

    [Fact]
    public async Task SetStarredAsync_TogglesStarred()
    {
        var feed = await CreateAndInsertFeedAsync();
        var article = CreateNewsArticle(feed.Id, "Test");
        article.IsStarred = false;
        var inserted = await _articleRepository.InsertAsync(article);

        await _articleRepository.SetStarredAsync(inserted.Id, true);
        var result = await _articleRepository.FindAsync(inserted.Id);

        Assert.True(result!.IsStarred);
    }

    [Fact]
    public async Task GetUnreadCountAsync_ReturnsCorrectCount()
    {
        var feed = await CreateAndInsertFeedAsync();

        var unread1 = CreateNewsArticle(feed.Id, "Unread 1", "guid-1");
        unread1.IsRead = false;
        await _articleRepository.InsertAsync(unread1);

        var unread2 = CreateNewsArticle(feed.Id, "Unread 2", "guid-2");
        unread2.IsRead = false;
        await _articleRepository.InsertAsync(unread2);

        var read = CreateNewsArticle(feed.Id, "Read", "guid-3");
        read.IsRead = true;
        await _articleRepository.InsertAsync(read);

        var count = await _articleRepository.GetUnreadCountAsync();

        Assert.Equal(2, count);
    }

    [Fact]
    public async Task MarkAllReadByFeedAsync_MarksAllArticlesInFeedAsRead()
    {
        var feed = await CreateAndInsertFeedAsync();

        var article1 = CreateNewsArticle(feed.Id, "Article 1", "guid-1");
        article1.IsRead = false;
        await _articleRepository.InsertAsync(article1);

        var article2 = CreateNewsArticle(feed.Id, "Article 2", "guid-2");
        article2.IsRead = false;
        await _articleRepository.InsertAsync(article2);

        await _articleRepository.MarkAllReadByFeedAsync(feed.Id);

        var articles = await _articleRepository.GetByFeedIdAsync(feed.Id);
        Assert.All(articles, a => Assert.True(a.IsRead));
    }

    private async Task<NewsFeed> CreateAndInsertFeedAsync(string feedUrl = "https://example.com/feed.xml")
    {
        var feed = new NewsFeed
        {
            Title = "Test Feed",
            FeedUrl = feedUrl,
            Monitored = true,
            QualityProfileId = 1,
            RefreshInterval = 60,
            Added = DateTime.UtcNow
        };
        return await _feedRepository.InsertAsync(feed);
    }

    private static NewsArticle CreateNewsArticle(
        int feedId,
        string title = "Test Article",
        string? guid = null)
    {
        return new NewsArticle
        {
            NewsFeedId = feedId,
            Title = title,
            ArticleGuid = guid ?? Guid.NewGuid().ToString(),
            PublishDate = DateTime.UtcNow,
            Added = DateTime.UtcNow
        };
    }
}
