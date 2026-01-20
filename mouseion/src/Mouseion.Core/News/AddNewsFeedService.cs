// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.News.RSS;

namespace Mouseion.Core.News;

public interface IAddNewsFeedService
{
    Task<NewsFeed> AddFeedAsync(string feedUrl, string? rootFolderPath = null, int qualityProfileId = 1, bool monitored = true, CancellationToken ct = default);
}

public class AddNewsFeedService : IAddNewsFeedService
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
            _logger.LogInformation("News feed with URL {FeedUrl} already exists", feedUrl);
            return existing;
        }

        var (parsedFeed, articles) = await _feedParser.ParseFeedAsync(feedUrl, ct).ConfigureAwait(false);

        parsedFeed.RootFolderPath = rootFolderPath;
        parsedFeed.QualityProfileId = qualityProfileId;
        parsedFeed.Monitored = monitored;

        var insertedFeed = await _feedRepository.InsertAsync(parsedFeed, ct).ConfigureAwait(false);
        _logger.LogInformation("Added news feed {Title} (ID: {Id})", insertedFeed.Title, insertedFeed.Id);

        foreach (var article in articles)
        {
            article.NewsFeedId = insertedFeed.Id;
            await _articleRepository.InsertAsync(article, ct).ConfigureAwait(false);
        }

        _logger.LogInformation("Added {Count} articles for feed {Title}", articles.Count, insertedFeed.Title);

        return insertedFeed;
    }
}
