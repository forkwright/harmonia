// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.News;
using Mouseion.Core.Tests.Repositories;

namespace Mouseion.Core.Tests.News;

public class NewsFeedRepositoryTests : RepositoryTestBase
{
    private readonly NewsFeedRepository _repository;

    public NewsFeedRepositoryTests()
    {
        _repository = new NewsFeedRepository(Database);
    }

    [Fact]
    public async Task InsertAsync_InsertsNewsFeed()
    {
        var feed = CreateNewsFeed("Tech News", "https://example.com/feed.xml");

        var result = await _repository.InsertAsync(feed);

        Assert.True(result.Id > 0);
        Assert.Equal("Tech News", result.Title);
    }

    [Fact]
    public async Task FindAsync_ReturnsFeedById()
    {
        var feed = CreateNewsFeed("Tech News", "https://example.com/feed.xml");
        var inserted = await _repository.InsertAsync(feed);

        var result = await _repository.FindAsync(inserted.Id);

        Assert.NotNull(result);
        Assert.Equal(inserted.Id, result.Id);
        Assert.Equal("Tech News", result.Title);
    }

    [Fact]
    public async Task FindByFeedUrlAsync_ReturnsMatchingFeed()
    {
        var feed = CreateNewsFeed("Tech News", "https://example.com/feed.xml");
        await _repository.InsertAsync(feed);

        var result = await _repository.FindByFeedUrlAsync("https://example.com/feed.xml");

        Assert.NotNull(result);
        Assert.Equal("https://example.com/feed.xml", result.FeedUrl);
    }

    [Fact]
    public async Task FindByFeedUrlAsync_ReturnsNullWhenNotFound()
    {
        var result = await _repository.FindByFeedUrlAsync("https://nonexistent.com/feed.xml");

        Assert.Null(result);
    }

    [Fact]
    public async Task GetMonitoredAsync_ReturnsOnlyMonitoredFeeds()
    {
        var monitoredFeed = CreateNewsFeed("Monitored", "https://example.com/feed1.xml", monitored: true);
        var unmonitoredFeed = CreateNewsFeed("Unmonitored", "https://example.com/feed2.xml", monitored: false);
        await _repository.InsertAsync(monitoredFeed);
        await _repository.InsertAsync(unmonitoredFeed);

        var result = await _repository.GetMonitoredAsync();

        Assert.Single(result);
        Assert.Equal("Monitored", result[0].Title);
    }

    [Fact]
    public async Task UpdateAsync_ModifiesFeed()
    {
        var feed = CreateNewsFeed("Original", "https://example.com/feed.xml");
        var inserted = await _repository.InsertAsync(feed);

        inserted.Title = "Updated";
        inserted.Monitored = false;
        var result = await _repository.UpdateAsync(inserted);

        Assert.Equal("Updated", result.Title);
        Assert.False(result.Monitored);
    }

    [Fact]
    public async Task DeleteAsync_RemovesFeed()
    {
        var feed = CreateNewsFeed("ToDelete", "https://example.com/feed.xml");
        var inserted = await _repository.InsertAsync(feed);

        await _repository.DeleteAsync(inserted.Id);
        var result = await _repository.FindAsync(inserted.Id);

        Assert.Null(result);
    }

    [Fact]
    public void FindByFeedUrl_ReturnsMatchingFeed()
    {
        var feed = CreateNewsFeed("Tech News", "https://example.com/sync.xml");
        _repository.Insert(feed);

        var result = _repository.FindByFeedUrl("https://example.com/sync.xml");

        Assert.NotNull(result);
        Assert.Equal("https://example.com/sync.xml", result.FeedUrl);
    }

    private static NewsFeed CreateNewsFeed(
        string title = "Test Feed",
        string feedUrl = "https://example.com/feed.xml",
        bool monitored = true,
        int qualityProfileId = 1)
    {
        return new NewsFeed
        {
            Title = title,
            FeedUrl = feedUrl,
            Monitored = monitored,
            QualityProfileId = qualityProfileId,
            RefreshInterval = 60,
            Added = DateTime.UtcNow
        };
    }
}
