// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ServiceModel.Syndication;
using System.Xml;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Podcasts.RSS;

public interface IRSSFeedParser
{
    Task<(PodcastShow Show, List<PodcastEpisode> Episodes)> ParseFeedAsync(string feedUrl, CancellationToken ct = default);
}

public partial class RSSFeedParser : IRSSFeedParser
{
    private readonly ILogger<RSSFeedParser> _logger;
    private readonly IHttpClientFactory _httpClientFactory;

    public RSSFeedParser(ILogger<RSSFeedParser> logger, IHttpClientFactory httpClientFactory)
    {
        _logger = logger;
        _httpClientFactory = httpClientFactory;
    }

    public async Task<(PodcastShow Show, List<PodcastEpisode> Episodes)> ParseFeedAsync(
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

            var feed = SyndicationFeed.Load(xmlReader);

            var show = new PodcastShow
            {
                Title = feed.Title?.Text ?? "Unknown Podcast",
                Description = feed.Description?.Text,
                FeedUrl = feedUrl,
                ImageUrl = feed.ImageUrl?.ToString(),
                Website = feed.Links?.FirstOrDefault()?.Uri.ToString(),
                Added = DateTime.UtcNow
            };

            var episodes = new List<PodcastEpisode>();
            foreach (var item in feed.Items)
            {
                var episode = new PodcastEpisode
                {
                    Title = item.Title?.Text ?? "Unknown Episode",
                    Description = item.Summary?.Text,
                    EpisodeGuid = item.Id,
                    PublishDate = item.PublishDate.DateTime,
                    Added = DateTime.UtcNow
                };

                // Extract enclosure (audio file URL)
                var enclosure = item.Links?.FirstOrDefault(l => l.RelationshipType == "enclosure");
                if (enclosure != null)
                {
                    episode.EnclosureUrl = enclosure.Uri.ToString();
                    episode.EnclosureLength = enclosure.Length;
                    episode.EnclosureType = enclosure.MediaType;
                }

                // Try to extract duration from iTunes extension
                var durationExt = item.ElementExtensions.FirstOrDefault(e =>
                    e.OuterName == "duration" && e.OuterNamespace == "http://www.itunes.com/dtds/podcast-1.0.dtd");
                if (durationExt != null)
                {
                    var durationStr = durationExt.GetObject<string>();
                    if (TryParseDuration(durationStr, out var seconds))
                    {
                        episode.Duration = seconds;
                    }
                }

                episodes.Add(episode);
            }

            LogParsedEpisodes(episodes.Count, feedUrl);

            return (show, episodes);
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

    [LoggerMessage(Level = LogLevel.Information, Message = "Parsing RSS feed: {FeedUrl}")]
    private partial void LogParsingFeed(string feedUrl);

    [LoggerMessage(Level = LogLevel.Information, Message = "Parsed {EpisodeCount} episodes from feed {FeedUrl}")]
    private partial void LogParsedEpisodes(int episodeCount, string feedUrl);

    [LoggerMessage(Level = LogLevel.Error, Message = "Network error fetching RSS feed: {FeedUrl}")]
    private partial void LogNetworkError(Exception ex, string feedUrl);

    [LoggerMessage(Level = LogLevel.Error, Message = "XML parsing error for RSS feed: {FeedUrl}")]
    private partial void LogXmlParsingError(Exception ex, string feedUrl);

    [LoggerMessage(Level = LogLevel.Error, Message = "Invalid RSS feed format: {FeedUrl}")]
    private partial void LogInvalidFeedFormat(Exception ex, string feedUrl);

    private static bool TryParseDuration(string? durationStr, out int seconds)
    {
        seconds = 0;
        if (string.IsNullOrWhiteSpace(durationStr))
            return false;

        // Handle HH:MM:SS or MM:SS format
        var parts = durationStr.Split(':');
        if (parts.Length == 3 &&
            int.TryParse(parts[0], out var hours) &&
            int.TryParse(parts[1], out var minutes) &&
            int.TryParse(parts[2], out var secs))
        {
            seconds = hours * 3600 + minutes * 60 + secs;
            return true;
        }
        else if (parts.Length == 2 &&
                 int.TryParse(parts[0], out var mins) &&
                 int.TryParse(parts[1], out var s))
        {
            seconds = mins * 60 + s;
            return true;
        }
        else if (int.TryParse(durationStr, out seconds))
        {
            return true;
        }

        return false;
    }
}
