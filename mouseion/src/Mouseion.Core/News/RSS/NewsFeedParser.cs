// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Security.Cryptography;
using System.ServiceModel.Syndication;
using System.Text;
using System.Xml;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.News.RSS;

public interface INewsFeedParser
{
    Task<(NewsFeed Feed, List<NewsArticle> Articles)> ParseFeedAsync(string feedUrl, CancellationToken ct = default);
}

public partial class NewsFeedParser : INewsFeedParser
{
    private readonly ILogger<NewsFeedParser> _logger;
    private readonly IHttpClientFactory _httpClientFactory;

    public NewsFeedParser(ILogger<NewsFeedParser> logger, IHttpClientFactory httpClientFactory)
    {
        _logger = logger;
        _httpClientFactory = httpClientFactory;
    }

    public async Task<(NewsFeed Feed, List<NewsArticle> Articles)> ParseFeedAsync(
        string feedUrl,
        CancellationToken ct = default)
    {
        try
        {
            LogParsingFeed(feedUrl);

            var httpClient = _httpClientFactory.CreateClient();
            var response = await httpClient.GetStringAsync(feedUrl, ct).ConfigureAwait(false);

            using var stringReader = new StringReader(response);
            using var xmlReader = XmlReader.Create(stringReader);

            var syndicationFeed = SyndicationFeed.Load(xmlReader);

            var feed = new NewsFeed
            {
                Title = syndicationFeed.Title?.Text ?? "Unknown Feed",
                Description = syndicationFeed.Description?.Text,
                FeedUrl = feedUrl,
                SiteUrl = syndicationFeed.Links?.FirstOrDefault(l => l.RelationshipType == "alternate")?.Uri.ToString()
                          ?? syndicationFeed.Links?.FirstOrDefault()?.Uri.ToString(),
                FaviconUrl = ExtractFaviconUrl(syndicationFeed),
                Language = syndicationFeed.Language,
                Category = string.Join(", ", syndicationFeed.Categories.Select(c => c.Name)),
                Added = DateTime.UtcNow,
                LastFetchTime = DateTime.UtcNow
            };

            var articles = new List<NewsArticle>();
            foreach (var item in syndicationFeed.Items)
            {
                var content = item.Content is TextSyndicationContent textContent
                    ? textContent.Text
                    : item.Summary?.Text;

                var article = new NewsArticle
                {
                    Title = item.Title?.Text ?? "Untitled",
                    Description = item.Summary?.Text,
                    Content = content,
                    ContentHash = ComputeHash(content),
                    ArticleGuid = item.Id ?? item.Links?.FirstOrDefault()?.Uri.ToString(),
                    SourceUrl = item.Links?.FirstOrDefault()?.Uri.ToString(),
                    Author = string.Join(", ", item.Authors.Select(a => a.Name ?? a.Email)),
                    PublishDate = item.PublishDate != DateTimeOffset.MinValue
                        ? item.PublishDate.UtcDateTime
                        : item.LastUpdatedTime != DateTimeOffset.MinValue
                            ? item.LastUpdatedTime.UtcDateTime
                            : null,
                    ImageUrl = ExtractImageUrl(item),
                    Categories = string.Join(", ", item.Categories.Select(c => c.Name)),
                    Added = DateTime.UtcNow
                };

                articles.Add(article);
            }

            feed.ItemCount = articles.Count;
            feed.LastItemDate = articles.Max(a => a.PublishDate);

            LogParsedArticles(articles.Count, feedUrl);

            return (feed, articles);
        }
        catch (HttpRequestException ex)
        {
            LogNetworkError(ex, feedUrl);
            throw;
        }
        catch (XmlException ex)
        {
            LogXmlParsingError(ex, feedUrl);
            throw;
        }
        catch (InvalidOperationException ex)
        {
            LogInvalidFeedFormat(ex, feedUrl);
            throw;
        }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Parsing news feed: {FeedUrl}")]
    private partial void LogParsingFeed(string feedUrl);

    [LoggerMessage(Level = LogLevel.Information, Message = "Parsed {ArticleCount} articles from feed {FeedUrl}")]
    private partial void LogParsedArticles(int articleCount, string feedUrl);

    [LoggerMessage(Level = LogLevel.Error, Message = "Network error fetching news feed: {FeedUrl}")]
    private partial void LogNetworkError(Exception ex, string feedUrl);

    [LoggerMessage(Level = LogLevel.Error, Message = "XML parsing error for news feed: {FeedUrl}")]
    private partial void LogXmlParsingError(Exception ex, string feedUrl);

    [LoggerMessage(Level = LogLevel.Error, Message = "Invalid feed format: {FeedUrl}")]
    private partial void LogInvalidFeedFormat(Exception ex, string feedUrl);

    private static string? ExtractFaviconUrl(SyndicationFeed feed)
    {
        var imageUrl = feed.ImageUrl?.ToString();
        if (!string.IsNullOrEmpty(imageUrl))
            return imageUrl;

        var siteUrl = feed.Links?.FirstOrDefault()?.Uri;
        if (siteUrl != null)
        {
            return $"{siteUrl.Scheme}://{siteUrl.Host}/favicon.ico";
        }

        return null;
    }

    private static string? ExtractImageUrl(SyndicationItem item)
    {
        var mediaExt = item.ElementExtensions.FirstOrDefault(e =>
            e.OuterName == "thumbnail" && e.OuterNamespace == "http://search.yahoo.com/mrss/");
        if (mediaExt != null)
        {
            try
            {
                var element = mediaExt.GetObject<System.Xml.Linq.XElement>();
                return element.Attribute("url")?.Value;
            }
            catch
            {
            }
        }

        var enclosure = item.Links?.FirstOrDefault(l =>
            l.RelationshipType == "enclosure" &&
            l.MediaType?.StartsWith("image/") == true);
        if (enclosure != null)
        {
            return enclosure.Uri.ToString();
        }

        return null;
    }

    private static string? ComputeHash(string? content)
    {
        if (string.IsNullOrEmpty(content))
            return null;

        var bytes = SHA256.HashData(Encoding.UTF8.GetBytes(content));
        return Convert.ToHexString(bytes).ToLowerInvariant();
    }
}
