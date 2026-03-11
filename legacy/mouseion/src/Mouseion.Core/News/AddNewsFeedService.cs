// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.News.RSS;

namespace Mouseion.Core.News;

public interface IAddNewsFeedService
{
    Task<NewsFeed> AddFeedAsync(string feedUrl, string? rootFolderPath = null, int qualityProfileId = 1, bool monitored = true, CancellationToken ct = default);
}

public partial class AddNewsFeedService : IAddNewsFeedService
{
    private readonly INewsFeedRepository _feedRepository;
    private readonly INewsArticleRepository _articleRepository;
    private readonly INewsFeedParser _feedParser;
    private readonly ILogger<AddNewsFeedService> _logger;

    public AddNewsFeedService(
        INewsFeedRepository feedRepository,
        INewsArticleRepository articleRepository,
        INewsFeedParser feedParser,
        ILogger<AddNewsFeedService> logger)
    {
        _feedRepository = feedRepository;
        _articleRepository = articleRepository;
        _feedParser = feedParser;
        _logger = logger;
    }

    public async Task<NewsFeed> AddFeedAsync(
        string feedUrl,
        string? rootFolderPath = null,
        int qualityProfileId = 1,
        bool monitored = true,
        CancellationToken ct = default)
    {
        var existing = await _feedRepository.FindByFeedUrlAsync(feedUrl, ct).ConfigureAwait(false);
        if (existing != null)
        {
            LogFeedAlreadyExists(feedUrl);
            return existing;
        }

        var (parsedFeed, articles) = await _feedParser.ParseFeedAsync(feedUrl, ct).ConfigureAwait(false);

        parsedFeed.RootFolderPath = rootFolderPath;
        parsedFeed.QualityProfileId = qualityProfileId;
        parsedFeed.Monitored = monitored;

        var insertedFeed = await _feedRepository.InsertAsync(parsedFeed, ct).ConfigureAwait(false);
        LogFeedAdded(insertedFeed.Title, insertedFeed.Id);

        foreach (var article in articles)
        {
            article.NewsFeedId = insertedFeed.Id;
            await _articleRepository.InsertAsync(article, ct).ConfigureAwait(false);
        }

        LogArticlesAdded(articles.Count, insertedFeed.Title);

        return insertedFeed;
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "News feed with URL {FeedUrl} already exists")]
    private partial void LogFeedAlreadyExists(string feedUrl);

    [LoggerMessage(Level = LogLevel.Information, Message = "Added news feed {Title} (ID: {Id})")]
    private partial void LogFeedAdded(string? title, int id);

    [LoggerMessage(Level = LogLevel.Information, Message = "Added {Count} articles for feed {Title}")]
    private partial void LogArticlesAdded(int count, string? title);
}
