// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.News.RSS;

namespace Mouseion.Core.News;

public interface IRefreshNewsFeedService
{
    Task<int> RefreshFeedAsync(int feedId, CancellationToken ct = default);
    Task<int> RefreshAllDueFeedsAsync(CancellationToken ct = default);
}

public class RefreshNewsFeedService : IRefreshNewsFeedService
{
    private readonly INewsFeedRepository _feedRepository;
    private readonly INewsArticleRepository _articleRepository;
    private readonly INewsFeedParser _feedParser;
    private readonly ILogger<RefreshNewsFeedService> _logger;

    public RefreshNewsFeedService(
        INewsFeedRepository feedRepository,
        INewsArticleRepository articleRepository,
        INewsFeedParser feedParser,
        ILogger<RefreshNewsFeedService> logger)
    {
        _feedRepository = feedRepository;
        _articleRepository = articleRepository;
        _feedParser = feedParser;
        _logger = logger;
    }

    public async Task<int> RefreshFeedAsync(int feedId, CancellationToken ct = default)
    {
        var feed = await _feedRepository.FindAsync(feedId, ct).ConfigureAwait(false);
        if (feed == null)
        {
            _logger.LogWarning("News feed {FeedId} not found", feedId);
            return 0;
        }

        return await RefreshFeedInternalAsync(feed, ct).ConfigureAwait(false);
    }

    public async Task<int> RefreshAllDueFeedsAsync(CancellationToken ct = default)
    {
        var dueFeeds = await _feedRepository.GetDueForRefreshAsync(ct).ConfigureAwait(false);
        var totalNewArticles = 0;

        foreach (var feed in dueFeeds)
        {
            try
            {
                totalNewArticles += await RefreshFeedInternalAsync(feed, ct).ConfigureAwait(false);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error refreshing feed {FeedId} ({Title})", feed.Id, feed.Title);
            }
        }

        return totalNewArticles;
    }

    private async Task<int> RefreshFeedInternalAsync(NewsFeed feed, CancellationToken ct)
    {
        _logger.LogInformation("Refreshing news feed {FeedId} ({Title})", feed.Id, feed.Title);

        var (_, articles) = await _feedParser.ParseFeedAsync(feed.FeedUrl, ct).ConfigureAwait(false);

        var newArticleCount = 0;
        foreach (var article in articles)
        {
            if (string.IsNullOrEmpty(article.ArticleGuid))
                continue;

            var existing = await _articleRepository.FindByGuidAndFeedAsync(article.ArticleGuid, feed.Id, ct).ConfigureAwait(false);
            if (existing != null)
                continue;

            article.NewsFeedId = feed.Id;
            await _articleRepository.InsertAsync(article, ct).ConfigureAwait(false);
            newArticleCount++;
        }

        feed.LastFetchTime = DateTime.UtcNow;
        feed.ItemCount = await _articleRepository.CountAsync(ct).ConfigureAwait(false);
        feed.LastItemDate = articles.Max(a => a.PublishDate);
        await _feedRepository.UpdateAsync(feed, ct).ConfigureAwait(false);

        _logger.LogInformation("Added {Count} new articles for feed {Title}", newArticleCount, feed.Title);

        return newArticleCount;
    }
}
