// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ServiceModel.Syndication;
using System.Xml;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Podcasts.RSS;

public interface IRSSFeedParser
{
    Task<(PodcastShow Show, List<PodcastEpisode> Episodes)> ParseFeedAsync(string feedUrl, CancellationToken ct = default);
    (PodcastShow Show, List<PodcastEpisode> Episodes) ParseFeed(string feedUrl);
}

public class RSSFeedParser : IRSSFeedParser
{
    private readonly ILogger<RSSFeedParser> _logger;
    private static readonly HttpClient _httpClient = new();

    public RSSFeedParser(ILogger<RSSFeedParser> logger)
    {
        _logger = logger;
    }

    public async Task<(PodcastShow Show, List<PodcastEpisode> Episodes)> ParseFeedAsync(
        string feedUrl,
        CancellationToken ct = default)
    {
        try
        {
            _logger.LogInformation("Parsing RSS feed: {FeedUrl}", feedUrl);

            var response = await _httpClient.GetStringAsync(feedUrl, ct).ConfigureAwait(false);

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

            _logger.LogInformation("Parsed {EpisodeCount} episodes from feed {FeedUrl}", episodes.Count, feedUrl);

            return (show, episodes);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to parse RSS feed: {FeedUrl}", feedUrl);
            throw;
        }
    }

    public (PodcastShow Show, List<PodcastEpisode> Episodes) ParseFeed(string feedUrl)
    {
        return ParseFeedAsync(feedUrl).GetAwaiter().GetResult();
    }

    private static bool TryParseDuration(string? durationStr, out int seconds)
    {
        seconds = 0;
        if (string.IsNullOrWhiteSpace(durationStr))
            return false;

        // Handle HH:MM:SS or MM:SS format
        var parts = durationStr.Split(':');
        if (parts.Length == 3)
        {
            if (int.TryParse(parts[0], out var hours) &&
                int.TryParse(parts[1], out var minutes) &&
                int.TryParse(parts[2], out var secs))
            {
                seconds = hours * 3600 + minutes * 60 + secs;
                return true;
            }
        }
        else if (parts.Length == 2)
        {
            if (int.TryParse(parts[0], out var minutes) &&
                int.TryParse(parts[1], out var secs))
            {
                seconds = minutes * 60 + secs;
                return true;
            }
        }
        else if (int.TryParse(durationStr, out seconds))
        {
            return true;
        }

        return false;
    }
}
