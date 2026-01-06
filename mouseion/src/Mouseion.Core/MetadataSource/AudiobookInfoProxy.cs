// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http;
using System.Text.Json;
using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Common.Http;
using Mouseion.Core.Audiobooks;

namespace Mouseion.Core.MetadataSource;

/// <summary>
/// Audnexus metadata provider for audiobooks
/// </summary>
public class AudiobookInfoProxy : IProvideAudiobookInfo
{
    private const string BaseUrl = "https://api.audnex.us";
    private const string BooksUrl = "https://api.audnex.us/books";
    private const string UserAgent = "Mouseion/1.0 (https://github.com/forkwright/mouseion)";

    private readonly IHttpClient _httpClient;
    private readonly IMemoryCache _cache;
    private readonly ILogger<AudiobookInfoProxy> _logger;

    public AudiobookInfoProxy(IHttpClient httpClient, IMemoryCache cache, ILogger<AudiobookInfoProxy> logger)
    {
        _httpClient = httpClient;
        _cache = cache;
        _logger = logger;
    }

    public async Task<Audiobook?> GetByAsinAsync(string asin, CancellationToken ct = default)
    {
        var cacheKey = $"audiobook_asin_{asin}";

        if (_cache.TryGetValue(cacheKey, out Audiobook? cached))
        {
            _logger.LogDebug("Cache hit for audiobook ASIN: {Asin}", asin.SanitizeForLog());
            return cached;
        }

        try
        {
            _logger.LogDebug("Fetching audiobook by ASIN: {Asin}", asin.SanitizeForLog());

            var url = $"{BooksUrl}/{asin}";
            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("Audnexus returned {StatusCode} for ASIN {Asin}", response.StatusCode, asin.SanitizeForLog());
                return null;
            }

            var result = ParseAudiobook(response.Content, asin);
            if (result != null)
            {
                _cache.Set(cacheKey, result, TimeSpan.FromMinutes(15));
            }

            return result;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error fetching audiobook by ASIN: {Asin}", asin.SanitizeForLog());
            return null;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse response for audiobook ASIN: {Asin}", asin.SanitizeForLog());
            return null;
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: audiobook by ASIN {Asin}", asin.SanitizeForLog());
            return null;
        }
    }

    public async Task<Audiobook?> GetByIdAsync(int id, CancellationToken ct = default)
    {
        _logger.LogDebug("GetById not supported by Audnexus (uses ASIN)");
        return await Task.FromResult<Audiobook?>(null).ConfigureAwait(false);
    }

    public async Task<List<Audiobook>> SearchByTitleAsync(string title, CancellationToken ct = default)
    {
        var cacheKey = $"audiobook_search_title_{title.ToLowerInvariant()}";

        if (_cache.TryGetValue(cacheKey, out List<Audiobook>? cached))
        {
            _logger.LogDebug("Cache hit for audiobook title search: {Title}", title.SanitizeForLog());
            return cached ?? new List<Audiobook>();
        }

        try
        {
            _logger.LogDebug("Searching audiobooks by title: {Title}", title.SanitizeForLog());

            var url = $"{BooksUrl}?title={Uri.EscapeDataString(title)}";
            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("Audnexus search returned {StatusCode}", response.StatusCode);
                return new List<Audiobook>();
            }

            var result = ParseSearchResults(response.Content);
            _cache.Set(cacheKey, result, TimeSpan.FromMinutes(15));

            return result;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error searching audiobooks by title: {Title}", title.SanitizeForLog());
            return new List<Audiobook>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse search results for audiobook title: {Title}", title.SanitizeForLog());
            return new List<Audiobook>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: audiobook title search {Title}", title.SanitizeForLog());
            return new List<Audiobook>();
        }
    }

    public async Task<List<Audiobook>> SearchByAuthorAsync(string author, CancellationToken ct = default)
    {
        try
        {
            _logger.LogDebug("Searching audiobooks by author: {Author}", author.SanitizeForLog());

            var url = $"{BooksUrl}?author={Uri.EscapeDataString(author)}";
            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("Audnexus search returned {StatusCode}", response.StatusCode);
                return new List<Audiobook>();
            }

            return ParseSearchResults(response.Content);
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error searching audiobooks by author: {Author}", author.SanitizeForLog());
            return new List<Audiobook>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse search results for audiobook author: {Author}", author.SanitizeForLog());
            return new List<Audiobook>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: audiobook author search {Author}", author.SanitizeForLog());
            return new List<Audiobook>();
        }
    }

    public async Task<List<Audiobook>> SearchByNarratorAsync(string narrator, CancellationToken ct = default)
    {
        try
        {
            _logger.LogDebug("Searching audiobooks by narrator: {Narrator}", narrator.SanitizeForLog());

            var url = $"{BooksUrl}?narrator={Uri.EscapeDataString(narrator)}";
            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("Audnexus search returned {StatusCode}", response.StatusCode);
                return new List<Audiobook>();
            }

            return ParseSearchResults(response.Content);
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error searching audiobooks by narrator: {Narrator}", narrator.SanitizeForLog());
            return new List<Audiobook>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse search results for audiobook narrator: {Narrator}", narrator.SanitizeForLog());
            return new List<Audiobook>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: audiobook narrator search {Narrator}", narrator.SanitizeForLog());
            return new List<Audiobook>();
        }
    }

    public async Task<List<Audiobook>> GetTrendingAsync(CancellationToken ct = default)
    {
        _logger.LogDebug("GetTrending not supported by Audnexus");
        return await Task.FromResult(new List<Audiobook>()).ConfigureAwait(false);
    }

    public async Task<List<Audiobook>> GetPopularAsync(CancellationToken ct = default)
    {
        _logger.LogDebug("GetPopular not supported by Audnexus");
        return await Task.FromResult(new List<Audiobook>()).ConfigureAwait(false);
    }

    private Audiobook? ParseAudiobook(string json, string asin)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            var audiobook = new Audiobook
            {
                Title = root.GetProperty("title").GetString() ?? "Unknown",
                Year = ExtractYear(root),
                Metadata = new AudiobookMetadata
                {
                    Description = GetStringProperty(root, "summary"),
                    ForeignAudiobookId = asin,
                    AudnexusId = asin,
                    AudibleId = asin,
                    Asin = asin,
                    ReleaseDate = ExtractDate(root),
                    Publisher = GetStringProperty(root, "publisher"),
                    Language = GetStringProperty(root, "language"),
                    Narrator = GetStringProperty(root, "narrator"),
                    Narrators = GetStringArrayProperty(root, "narrators"),
                    DurationMinutes = ExtractDuration(root),
                    IsAbridged = GetBoolProperty(root, "isAbridged"),
                    Genres = GetStringArrayProperty(root, "genres")
                }
            };

            return audiobook;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing Audnexus audiobook JSON");
            return null;
        }
    }

    private List<Audiobook> ParseSearchResults(string json)
    {
        var audiobooks = new List<Audiobook>();

        try
        {
            using var doc = JsonDocument.Parse(json);

            // Handle both array and object responses
            if (doc.RootElement.ValueKind == JsonValueKind.Array)
            {
                foreach (var item in doc.RootElement.EnumerateArray())
                {
                    var audiobook = ParseSearchResult(item);
                    if (audiobook != null)
                    {
                        audiobooks.Add(audiobook);
                    }
                }
            }
            else if (doc.RootElement.TryGetProperty("results", out var results))
            {
                foreach (var item in results.EnumerateArray())
                {
                    var audiobook = ParseSearchResult(item);
                    if (audiobook != null)
                    {
                        audiobooks.Add(audiobook);
                    }
                }
            }
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing Audnexus search results");
        }

        return audiobooks;
    }

    private Audiobook? ParseSearchResult(JsonElement item)
    {
        try
        {
            var asin = GetStringProperty(item, "asin");
            if (string.IsNullOrWhiteSpace(asin))
            {
                return null;
            }

            var audiobook = new Audiobook
            {
                Title = item.GetProperty("title").GetString() ?? "Unknown",
                Year = ExtractYear(item),
                Metadata = new AudiobookMetadata
                {
                    ForeignAudiobookId = asin,
                    AudnexusId = asin,
                    AudibleId = asin,
                    Asin = asin,
                    Description = GetStringProperty(item, "summary"),
                    ReleaseDate = ExtractDate(item),
                    Publisher = GetStringProperty(item, "publisher"),
                    Language = GetStringProperty(item, "language"),
                    Narrator = GetStringProperty(item, "narrator"),
                    Narrators = GetStringArrayProperty(item, "narrators"),
                    DurationMinutes = ExtractDuration(item),
                    IsAbridged = GetBoolProperty(item, "isAbridged"),
                    Genres = GetStringArrayProperty(item, "genres")
                }
            };

            return audiobook;
        }
        catch (JsonException ex)
        {
            _logger.LogWarning(ex, "Error parsing individual search result");
            return null;
        }
    }

    private static string? GetStringProperty(JsonElement element, string propertyName)
    {
        if (element.TryGetProperty(propertyName, out var property) && property.ValueKind == JsonValueKind.String)
        {
            return property.GetString();
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
            else if (property.ValueKind == JsonValueKind.False)
            {
                return false;
            }
        }

        return false;
    }

    private static List<string> GetStringArrayProperty(JsonElement element, string propertyName)
    {
        var result = new List<string>();

        if (element.TryGetProperty(propertyName, out var property) && property.ValueKind == JsonValueKind.Array)
        {
            foreach (var item in property.EnumerateArray())
            {
                if (item.ValueKind == JsonValueKind.String)
                {
                    var value = item.GetString();
                    if (!string.IsNullOrWhiteSpace(value))
                    {
                        result.Add(value);
                    }
                }
            }
        }

        return result;
    }

    private static int ExtractYear(JsonElement element)
    {
        var releaseDate = ExtractDate(element);
        return releaseDate?.Year ?? 0;
    }

    private static DateTime? ExtractDate(JsonElement element)
    {
        var dateStr = GetStringProperty(element, "releaseDate");
        if (string.IsNullOrWhiteSpace(dateStr))
        {
            dateStr = GetStringProperty(element, "publishDate");
        }

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

    private static int? ExtractDuration(JsonElement element)
    {
        if (element.TryGetProperty("runtimeLengthMin", out var runtime) && runtime.ValueKind == JsonValueKind.Number)
        {
            return runtime.GetInt32();
        }

        if (element.TryGetProperty("duration", out var duration) && duration.ValueKind == JsonValueKind.Number)
        {
            return duration.GetInt32();
        }

        return null;
    }
}
