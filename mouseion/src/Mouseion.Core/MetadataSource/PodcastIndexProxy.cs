// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Http;
using Mouseion.Core.Podcasts;

namespace Mouseion.Core.MetadataSource;

public class PodcastIndexProxy : IProvidePodcastInfo
{
    private const string BaseUrl = "https://api.podcastindex.org/api/1.0";
    private readonly string _apiKey;
    private readonly string _apiSecret;

    private readonly IHttpClient _httpClient;
    private readonly IMemoryCache _cache;
    private readonly ILogger<PodcastIndexProxy> _logger;

    public PodcastIndexProxy(
        IHttpClient httpClient,
        IMemoryCache cache,
        IConfiguration configuration,
        ILogger<PodcastIndexProxy> logger)
    {
        _httpClient = httpClient;
        _cache = cache;
        _logger = logger;
        _apiKey = configuration["PodcastIndex:ApiKey"] ?? Environment.GetEnvironmentVariable("PODCASTINDEX_API_KEY") ?? string.Empty;
        _apiSecret = configuration["PodcastIndex:ApiSecret"] ?? Environment.GetEnvironmentVariable("PODCASTINDEX_API_SECRET") ?? string.Empty;

        if (string.IsNullOrWhiteSpace(_apiKey) || string.IsNullOrWhiteSpace(_apiSecret))
        {
            _logger.LogWarning("PodcastIndex API credentials not configured. Set PodcastIndex:ApiKey and PodcastIndex:ApiSecret in config or env vars");
        }
    }

    public async Task<List<PodcastShow>> SearchAsync(string query, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(_apiKey) || string.IsNullOrWhiteSpace(_apiSecret))
        {
            _logger.LogWarning("PodcastIndex API credentials not configured");
            return new List<PodcastShow>();
        }

        var cacheKey = $"podcastindex_search_{query}";

        if (_cache.TryGetValue(cacheKey, out List<PodcastShow>? cached))
        {
            _logger.LogDebug("Cache hit for PodcastIndex search: {Query}", query);
            return cached ?? new List<PodcastShow>();
        }

        try
        {
            _logger.LogDebug("Searching PodcastIndex for: {Query}", query);

            var url = $"{BaseUrl}/search/byterm?q={Uri.EscapeDataString(query)}";
            var request = BuildAuthenticatedRequest(url);

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("PodcastIndex returned {StatusCode} for query {Query}", response.StatusCode, query);
                return new List<PodcastShow>();
            }

            var results = ParseSearchResults(response.Content);
            _cache.Set(cacheKey, results, TimeSpan.FromMinutes(15));

            return results;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error searching PodcastIndex for: {Query}", query);
            return new List<PodcastShow>();
        }
    }

    public List<PodcastShow> Search(string query)
    {
        return SearchAsync(query).GetAwaiter().GetResult();
    }

    public async Task<PodcastShow?> GetByFeedUrlAsync(string feedUrl, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(_apiKey) || string.IsNullOrWhiteSpace(_apiSecret))
        {
            _logger.LogWarning("PodcastIndex API credentials not configured");
            return null;
        }

        var cacheKey = $"podcastindex_feed_{feedUrl}";

        if (_cache.TryGetValue(cacheKey, out PodcastShow? cached))
        {
            _logger.LogDebug("Cache hit for PodcastIndex feed URL: {FeedUrl}", feedUrl);
            return cached;
        }

        try
        {
            _logger.LogDebug("Fetching podcast by feed URL: {FeedUrl}", feedUrl);

            var url = $"{BaseUrl}/podcasts/byfeedurl?url={Uri.EscapeDataString(feedUrl)}";
            var request = BuildAuthenticatedRequest(url);

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("PodcastIndex returned {StatusCode} for feed URL {FeedUrl}", response.StatusCode, feedUrl);
                return null;
            }

            var result = ParsePodcast(response.Content);
            if (result != null)
            {
                _cache.Set(cacheKey, result, TimeSpan.FromMinutes(15));
            }

            return result;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error fetching podcast by feed URL: {FeedUrl}", feedUrl);
            return null;
        }
    }

    public PodcastShow? GetByFeedUrl(string feedUrl)
    {
        return GetByFeedUrlAsync(feedUrl).GetAwaiter().GetResult();
    }

    public async Task<PodcastShow?> GetByPodcastIndexIdAsync(int podcastIndexId, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(_apiKey) || string.IsNullOrWhiteSpace(_apiSecret))
        {
            _logger.LogWarning("PodcastIndex API credentials not configured");
            return null;
        }

        var cacheKey = $"podcastindex_id_{podcastIndexId}";

        if (_cache.TryGetValue(cacheKey, out PodcastShow? cached))
        {
            _logger.LogDebug("Cache hit for PodcastIndex ID: {PodcastIndexId}", podcastIndexId);
            return cached;
        }

        try
        {
            _logger.LogDebug("Fetching podcast by PodcastIndex ID: {PodcastIndexId}", podcastIndexId);

            var url = $"{BaseUrl}/podcasts/byfeedid?id={podcastIndexId}";
            var request = BuildAuthenticatedRequest(url);

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("PodcastIndex returned {StatusCode} for ID {PodcastIndexId}", response.StatusCode, podcastIndexId);
                return null;
            }

            var result = ParsePodcast(response.Content);
            if (result != null)
            {
                _cache.Set(cacheKey, result, TimeSpan.FromMinutes(15));
            }

            return result;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error fetching podcast by PodcastIndex ID: {PodcastIndexId}", podcastIndexId);
            return null;
        }
    }

    public PodcastShow? GetByPodcastIndexId(int podcastIndexId)
    {
        return GetByPodcastIndexIdAsync(podcastIndexId).GetAwaiter().GetResult();
    }

    private HttpRequest BuildAuthenticatedRequest(string url)
    {
        var unixTime = DateTimeOffset.UtcNow.ToUnixTimeSeconds().ToString();
        var authHash = ComputeAuthHash(_apiKey, _apiSecret, unixTime);

        var builder = new HttpRequestBuilder(url)
            .SetHeader("User-Agent", "Mouseion/1.0")
            .SetHeader("X-Auth-Date", unixTime)
            .SetHeader("X-Auth-Key", _apiKey)
            .SetHeader("Authorization", authHash);

        return builder.Build();
    }

    private static string ComputeAuthHash(string apiKey, string apiSecret, string unixTime)
    {
        var data = apiKey + apiSecret + unixTime;
        var hash = SHA1.HashData(Encoding.UTF8.GetBytes(data));
        return BitConverter.ToString(hash).Replace("-", "").ToLowerInvariant();
    }

    private List<PodcastShow> ParseSearchResults(string json)
    {
        var results = new List<PodcastShow>();

        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            if (root.TryGetProperty("feeds", out var feeds))
            {
                foreach (var feed in feeds.EnumerateArray())
                {
                    var show = ParsePodcastFromJson(feed);
                    if (show != null)
                    {
                        results.Add(show);
                    }
                }
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error parsing PodcastIndex search results");
        }

        return results;
    }

    private PodcastShow? ParsePodcast(string json)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            if (root.TryGetProperty("feed", out var feed))
            {
                return ParsePodcastFromJson(feed);
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error parsing PodcastIndex podcast");
        }

        return null;
    }

    private static PodcastShow? ParsePodcastFromJson(JsonElement feed)
    {
        var show = new PodcastShow
        {
            Title = feed.GetProperty("title").GetString() ?? "Unknown Podcast",
            Description = feed.TryGetProperty("description", out var desc) ? desc.GetString() : null,
            Author = feed.TryGetProperty("author", out var author) ? author.GetString() : null,
            FeedUrl = feed.GetProperty("url").GetString() ?? string.Empty,
            ImageUrl = feed.TryGetProperty("image", out var image) ? image.GetString() : null,
            Website = feed.TryGetProperty("link", out var link) ? link.GetString() : null,
            Language = feed.TryGetProperty("language", out var lang) ? lang.GetString() : null,
            ForeignPodcastId = feed.TryGetProperty("id", out var id) ? id.GetInt32().ToString() : null,
            ItunesId = feed.TryGetProperty("itunesId", out var itunesId) ? itunesId.GetInt32().ToString() : null,
            Added = DateTime.UtcNow
        };

        return show;
    }
}
