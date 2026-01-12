// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Web;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Common.Http;
using Mouseion.Core.Audiobooks;
using Mouseion.Core.Books;

namespace Mouseion.Core.Indexers.MyAnonamouse;

/// <summary>
/// MyAnonamouse torrent indexer for books and audiobooks
/// </summary>
public partial class MyAnonamouseIndexer : IMyAnonamouseIndexer
{
    private readonly IHttpClient _httpClient;
    private readonly MyAnonamouseSettings _settings;
    private readonly ILogger<MyAnonamouseIndexer> _logger;

    public string Name => "MyAnonamouse";
    public bool Enabled => _settings.Enabled;

    public MyAnonamouseIndexer(
        IHttpClient httpClient,
        MyAnonamouseSettings settings,
        ILogger<MyAnonamouseIndexer> logger)
    {
        _httpClient = httpClient;
        _settings = settings;
        _logger = logger;
    }

    public async Task<List<IndexerResult>> SearchBooksAsync(BookSearchCriteria criteria, CancellationToken ct = default)
    {
        var searchTerm = BuildSearchTerm(criteria.Author, criteria.Title);
        return await SearchAsync(searchTerm, "book", ct).ConfigureAwait(false);
    }

    public async Task<List<IndexerResult>> SearchAudiobooksAsync(AudiobookSearchCriteria criteria, CancellationToken ct = default)
    {
        var searchTerm = BuildSearchTerm(criteria.Author, criteria.Title);
        return await SearchAsync(searchTerm, "audiobook", ct).ConfigureAwait(false);
    }

    private string BuildSearchTerm(string? author, string? title)
    {
        var terms = new List<string>();

        if (!string.IsNullOrWhiteSpace(author))
        {
            terms.Add(author);
        }

        if (!string.IsNullOrWhiteSpace(title))
        {
            terms.Add(title);
        }

        return string.Join(" ", terms);
    }

    private async Task<List<IndexerResult>> SearchAsync(string searchTerm, string mediaType, CancellationToken ct = default)
    {
        var results = new List<IndexerResult>();

        try
        {
            if (string.IsNullOrWhiteSpace(searchTerm))
            {
                _logger.LogDebug("Empty search term, skipping search");
                return results;
            }

            var sanitizedTerm = SanitizeSearchQuery(searchTerm);
            if (string.IsNullOrWhiteSpace(sanitizedTerm))
            {
                _logger.LogDebug("Search term is empty after sanitization: {OriginalTerm}", searchTerm.SanitizeForLog());
                return results;
            }

            _logger.LogDebug("Searching MyAnonamouse for {MediaType}: {SearchTerm}", mediaType, sanitizedTerm);

            var searchUrl = BuildSearchUrl(sanitizedTerm);
            var request = new HttpRequestBuilder(searchUrl)
                .SetHeader("User-Agent", "Mouseion/1.0")
                .SetHeader("Cookie", $"mam_id={_settings.MamId}")
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("MyAnonamouse returned {StatusCode}", response.StatusCode);
                return results;
            }

            results = ParseResponse(response.Content);
            _logger.LogInformation("Found {Count} results for {SearchTerm}", results.Count, sanitizedTerm);
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error searching MyAnonamouse for: {SearchTerm}", searchTerm.SanitizeForLog());
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse MyAnonamouse response for: {SearchTerm}", searchTerm.SanitizeForLog());
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled searching MyAnonamouse for: {SearchTerm}", searchTerm.SanitizeForLog());
        }

        return results;
    }

    private string BuildSearchUrl(string searchTerm)
    {
        var searchType = _settings.SearchType switch
        {
            MyAnonamouseSearchType.Active => "active",
            MyAnonamouseSearchType.Freeleech => "fl",
            MyAnonamouseSearchType.FreeleechOrVip => "fl-VIP",
            MyAnonamouseSearchType.Vip => "VIP",
            MyAnonamouseSearchType.NotVip => "nVIP",
            _ => "all"
        };

        var queryParams = new Dictionary<string, string>
        {
            ["tor[text]"] = searchTerm,
            ["tor[searchType]"] = searchType,
            ["tor[srchIn][title]"] = "true",
            ["tor[srchIn][author]"] = "true",
            ["tor[srchIn][narrator]"] = "true",
            ["tor[searchIn]"] = "torrents",
            ["tor[sortType]"] = "default",
            ["tor[perpage]"] = "100",
            ["tor[startNumber]"] = "0",
            ["thumbnails"] = "1",
            ["description"] = "1",
            ["tor[cat][]"] = "0"
        };

        if (_settings.SearchInDescription)
        {
            queryParams["tor[srchIn][description]"] = "true";
        }

        if (_settings.SearchInSeries)
        {
            queryParams["tor[srchIn][series]"] = "true";
        }

        if (_settings.SearchInFilenames)
        {
            queryParams["tor[srchIn][filenames]"] = "true";
        }

        var queryString = string.Join("&", queryParams.Select(kvp =>
            $"{HttpUtility.UrlEncode(kvp.Key)}={HttpUtility.UrlEncode(kvp.Value)}"));

        return $"{_settings.BaseUrl.TrimEnd('/')}/tor/js/loadSearchJSONbasic.php?{queryString}";
    }

    private List<IndexerResult> ParseResponse(string json)
    {
        var results = new List<IndexerResult>();

        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            if (!root.TryGetProperty("data", out var data))
            {
                return results;
            }

            foreach (var item in data.EnumerateArray())
            {
                try
                {
                    var result = ParseTorrent(item);
                    if (result != null && result.Seeders >= _settings.MinimumSeeders)
                    {
                        results.Add(result);
                    }
                }
                catch (JsonException ex)
                {
                    _logger.LogWarning(ex, "Error parsing individual torrent result");
                }
            }
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing MyAnonamouse response");
        }

        return results;
    }

    private static IndexerResult? ParseTorrent(JsonElement item)
    {
        if (!item.TryGetProperty("id", out var idProp))
        {
            return null;
        }

        var result = new IndexerResult
        {
            TorrentId = idProp.GetString() ?? string.Empty,
            Title = GetStringProperty(item, "title") ?? "Unknown",
            Author = GetStringProperty(item, "author_info"),
            Category = GetStringProperty(item, "cat_name"),
            Size = GetLongProperty(item, "size") ?? 0,
            Seeders = GetIntProperty(item, "seeders") ?? 0,
            Leechers = GetIntProperty(item, "leechers") ?? 0,
            PublishDate = GetDateProperty(item, "added"),
            DownloadUrl = GetStringProperty(item, "download_link"),
            InfoUrl = GetStringProperty(item, "info_link"),
            IsFreeleech = GetBoolProperty(item, "free")
        };

        return result;
    }

    private static string? GetStringProperty(JsonElement element, string propertyName)
    {
        if (element.TryGetProperty(propertyName, out var property) && property.ValueKind == JsonValueKind.String)
        {
            return property.GetString();
        }

        return null;
    }

    private static int? GetIntProperty(JsonElement element, string propertyName)
    {
        if (element.TryGetProperty(propertyName, out var property))
        {
            if (property.ValueKind == JsonValueKind.Number)
            {
                return property.GetInt32();
            }
            else if (property.ValueKind == JsonValueKind.String)
            {
                var str = property.GetString();
                if (int.TryParse(str, out var value))
                {
                    return value;
                }
            }
        }

        return null;
    }

    private static long? GetLongProperty(JsonElement element, string propertyName)
    {
        if (element.TryGetProperty(propertyName, out var property))
        {
            if (property.ValueKind == JsonValueKind.Number)
            {
                return property.GetInt64();
            }
            else if (property.ValueKind == JsonValueKind.String)
            {
                var str = property.GetString();
                if (long.TryParse(str, out var value))
                {
                    return value;
                }
            }
        }

        return null;
    }

    private static bool GetBoolProperty(JsonElement element, string propertyName)
    {
        if (element.TryGetProperty(propertyName, out var property))
        {
            if (property.ValueKind == JsonValueKind.True)
            {
                return true;
            }
            else if (property.ValueKind == JsonValueKind.String)
            {
                var str = property.GetString();
                return str?.Equals("1", StringComparison.Ordinal) == true;
            }
        }

        return false;
    }

    private static DateTime? GetDateProperty(JsonElement element, string propertyName)
    {
        var dateStr = GetStringProperty(element, propertyName);
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

    private static string SanitizeSearchQuery(string query)
    {
        return SanitizeRegex().Replace(query, " ").Trim();
    }

    [GeneratedRegex("[^\\w]+", RegexOptions.IgnoreCase | RegexOptions.Compiled, 1000)]
    private static partial Regex SanitizeRegex();
}

public class IndexerResult
{
    public string TorrentId { get; set; } = string.Empty;
    public string Title { get; set; } = string.Empty;
    public string? Author { get; set; }
    public string? Category { get; set; }
    public long Size { get; set; }
    public int Seeders { get; set; }
    public int Leechers { get; set; }
    public DateTime? PublishDate { get; set; }
    public string? DownloadUrl { get; set; }
    public string? InfoUrl { get; set; }
    public bool IsFreeleech { get; set; }
}
