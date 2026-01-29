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

public partial class RefreshNewsFeedService : IRefreshNewsFeedService
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
            LogFeedNotFound(feedId);
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
                LogRefreshError(ex, feed.Id, feed.Title);
            }
        }

        return totalNewArticles;
    }

    private async Task<int> RefreshFeedInternalAsync(NewsFeed feed, CancellationToken ct)
    {
        LogRefreshingFeed(feed.Id, feed.Title);

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

        LogArticlesAdded(newArticleCount, feed.Title);

        return newArticleCount;
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "News feed {FeedId} not found")]
    private partial void LogFeedNotFound(int feedId);

    [LoggerMessage(Level = LogLevel.Error, Message = "Error refreshing feed {FeedId} ({Title})")]
    private partial void LogRefreshError(Exception ex, int feedId, string? title);

    [LoggerMessage(Level = LogLevel.Information, Message = "Refreshing news feed {FeedId} ({Title})")]
    private partial void LogRefreshingFeed(int feedId, string? title);

    [LoggerMessage(Level = LogLevel.Information, Message = "Added {Count} new articles for feed {Title}")]
    private partial void LogArticlesAdded(int count, string? title);
}
