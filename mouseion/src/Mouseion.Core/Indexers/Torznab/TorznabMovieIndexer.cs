// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Web;
using System.Xml.Linq;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Http;

namespace Mouseion.Core.Indexers.Torznab;

public class TorznabMovieIndexer
{
    private readonly IHttpClient _httpClient;
    private readonly TorznabSettings _settings;
    private readonly ILogger<TorznabMovieIndexer> _logger;

    public string Name => "Torznab Movie";
    public bool Enabled => _settings.Enabled;

    public TorznabMovieIndexer(
        IHttpClient httpClient,
        TorznabSettings settings,
        ILogger<TorznabMovieIndexer> logger)
    {
        _httpClient = httpClient;
        _settings = settings;
        _logger = logger;
    }

    public async Task<List<TorznabRelease>> SearchAsync(string query, CancellationToken ct = default)
    {
        try
        {
            _logger.LogDebug("Searching Torznab movie indexer for: {Query}", query);

            var searchUrl = BuildSearchUrl(query);
            var request = new HttpRequestBuilder(searchUrl)
                .SetHeader("User-Agent", "Mouseion/1.0")
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("Torznab indexer returned {StatusCode}", response.StatusCode);
                return new List<TorznabRelease>();
            }

            var releases = ParseTorznabResponse(response.Content);
            _logger.LogInformation("Found {Count} releases for {Query}", releases.Count, query);
            return releases;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error searching Torznab movie indexer for: {Query}", query);
            return new List<TorznabRelease>();
        }
    }

    public List<TorznabRelease> Search(string query)
    {
        try
        {
            _logger.LogDebug("Searching Torznab movie indexer for: {Query}", query);

            var searchUrl = BuildSearchUrl(query);
            var request = new HttpRequestBuilder(searchUrl)
                .SetHeader("User-Agent", "Mouseion/1.0")
                .Build();

            var response = _httpClient.Get(request);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("Torznab indexer returned {StatusCode}", response.StatusCode);
                return new List<TorznabRelease>();
            }

            var releases = ParseTorznabResponse(response.Content);
            _logger.LogInformation("Found {Count} releases for {Query}", releases.Count, query);
            return releases;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error searching Torznab movie indexer for: {Query}", query);
            return new List<TorznabRelease>();
        }
    }

    private string BuildSearchUrl(string query)
    {
        var queryParams = new Dictionary<string, string>
        {
            ["t"] = "movie",
            ["q"] = query,
            ["apikey"] = _settings.ApiKey,
            ["cat"] = "2000" // Movies category
        };

        if (_settings.MinimumSeeders > 0)
        {
            queryParams["minseeders"] = _settings.MinimumSeeders.ToString();
        }

        var queryString = string.Join("&", queryParams.Select(kvp =>
            $"{HttpUtility.UrlEncode(kvp.Key)}={HttpUtility.UrlEncode(kvp.Value)}"));

        return $"{_settings.BaseUrl.TrimEnd('/')}/api?{queryString}";
    }

    private List<TorznabRelease> ParseTorznabResponse(string xml)
    {
        var releases = new List<TorznabRelease>();

        try
        {
            var doc = XDocument.Parse(xml);
            var channel = doc.Root?.Element("channel");
            if (channel == null)
            {
                return releases;
            }

            var items = channel.Elements("item");
            foreach (var item in items)
            {
                try
                {
                    var release = ParseItem(item);
                    if (release != null)
                    {
                        releases.Add(release);
                    }
                }
                catch (Exception ex)
                {
                    _logger.LogWarning(ex, "Error parsing Torznab item");
                }
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error parsing Torznab response");
        }

        return releases;
    }

    private static TorznabRelease? ParseItem(XElement item)
    {
        var title = item.Element("title")?.Value;
        if (string.IsNullOrEmpty(title))
        {
            return null;
        }

        var release = new TorznabRelease
        {
            Title = title,
            Size = ParseTorznabAttribute(item, "size") ?? 0,
            PublishDate = ParseDate(item.Element("pubDate")?.Value),
            DownloadUrl = item.Element("link")?.Value,
            InfoUrl = item.Element("comments")?.Value,
            Seeders = (int)(ParseTorznabAttribute(item, "seeders") ?? 0),
            Peers = (int)(ParseTorznabAttribute(item, "peers") ?? 0)
        };

        return release;
    }

    private static long? ParseTorznabAttribute(XElement item, string name)
    {
        var ns = XNamespace.Get("http://torznab.com/schemas/2015/feed");
        var attr = item.Elements(ns + "attr")
            .FirstOrDefault(e => e.Attribute("name")?.Value == name);

        if (attr != null && long.TryParse(attr.Attribute("value")?.Value, out var value))
        {
            return value;
        }

        return null;
    }

    private static DateTime? ParseDate(string? dateStr)
    {
        if (string.IsNullOrWhiteSpace(dateStr))
        {
            return null;
        }

        if (DateTime.TryParse(dateStr, out var date))
        {
            return date;
        }

        return null;
    }
}
