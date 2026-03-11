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
using Mouseion.Core.Books;

namespace Mouseion.Core.MetadataSource;

/// <summary>
/// OpenLibrary metadata provider for books
/// </summary>
public class BookInfoProxy : IProvideBookInfo
{
    private const string BaseUrl = "https://openlibrary.org";
    private const string SearchUrl = "https://openlibrary.org/search.json";
    private const string UserAgent = "Mouseion/1.0 (https://github.com/forkwright/mouseion)";

    private readonly IHttpClient _httpClient;
    private readonly IMemoryCache _cache;
    private readonly ILogger<BookInfoProxy> _logger;

    public BookInfoProxy(IHttpClient httpClient, IMemoryCache cache, ILogger<BookInfoProxy> logger)
    {
        _httpClient = httpClient;
        _cache = cache;
        _logger = logger;
    }

    public async Task<Book?> GetByExternalIdAsync(string externalId, CancellationToken ct = default)
    {
        var cacheKey = $"book_external_{externalId}";

        if (_cache.TryGetValue(cacheKey, out Book? cached))
        {
            _logger.LogDebug("Cache hit for book external ID: {ExternalId}", externalId.SanitizeForLog());
            return cached;
        }

        try
        {
            _logger.LogDebug("Fetching book by external ID: {ExternalId}", externalId.SanitizeForLog());

            var workId = externalId.StartsWith("/works/") ? externalId : $"/works/{externalId}";
            var url = $"{BaseUrl}{workId}.json";

            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("OpenLibrary returned {StatusCode} for work {WorkId}", response.StatusCode, workId.SanitizeForLog());
                return null;
            }

            var result = ParseWork(response.Content);
            if (result != null)
            {
                _cache.Set(cacheKey, result, TimeSpan.FromMinutes(15));
            }

            return result;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error fetching book by external ID: {ExternalId}", externalId.SanitizeForLog());
            return null;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse response for book external ID: {ExternalId}", externalId.SanitizeForLog());
            return null;
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: book by external ID {ExternalId}", externalId.SanitizeForLog());
            return null;
        }
    }

    public async Task<Book?> GetByIdAsync(int id, CancellationToken ct = default)
    {
        _logger.LogDebug("GetById not supported by OpenLibrary (uses string IDs)");
        return await Task.FromResult<Book?>(null).ConfigureAwait(false);
    }

    public async Task<List<Book>> SearchByTitleAsync(string title, CancellationToken ct = default)
    {
        var cacheKey = $"book_search_title_{title.ToLowerInvariant()}";

        if (_cache.TryGetValue(cacheKey, out List<Book>? cached))
        {
            _logger.LogDebug("Cache hit for book title search: {Title}", title.SanitizeForLog());
            return cached ?? new List<Book>();
        }

        try
        {
            _logger.LogDebug("Searching books by title: {Title}", title.SanitizeForLog());

            var url = $"{SearchUrl}?title={Uri.EscapeDataString(title)}&limit=20";
            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("OpenLibrary search returned {StatusCode}", response.StatusCode);
                return new List<Book>();
            }

            var result = ParseSearchResults(response.Content);
            _cache.Set(cacheKey, result, TimeSpan.FromMinutes(15));

            return result;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error searching books by title: {Title}", title.SanitizeForLog());
            return new List<Book>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse search results for book title: {Title}", title.SanitizeForLog());
            return new List<Book>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: book title search {Title}", title.SanitizeForLog());
            return new List<Book>();
        }
    }

    public async Task<List<Book>> SearchByAuthorAsync(string author, CancellationToken ct = default)
    {
        try
        {
            _logger.LogDebug("Searching books by author: {Author}", author.SanitizeForLog());

            var url = $"{SearchUrl}?author={Uri.EscapeDataString(author)}&limit=20";
            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("OpenLibrary search returned {StatusCode}", response.StatusCode);
                return new List<Book>();
            }

            return ParseSearchResults(response.Content);
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error searching books by author: {Author}", author.SanitizeForLog());
            return new List<Book>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse search results for book author: {Author}", author.SanitizeForLog());
            return new List<Book>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: book author search {Author}", author.SanitizeForLog());
            return new List<Book>();
        }
    }

    public async Task<List<Book>> SearchByIsbnAsync(string isbn, CancellationToken ct = default)
    {
        try
        {
            _logger.LogDebug("Searching books by ISBN: {Isbn}", isbn.SanitizeForLog());

            var url = $"{SearchUrl}?isbn={Uri.EscapeDataString(isbn)}&limit=5";
            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("OpenLibrary search returned {StatusCode}", response.StatusCode);
                return new List<Book>();
            }

            return ParseSearchResults(response.Content);
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error searching books by ISBN: {Isbn}", isbn.SanitizeForLog());
            return new List<Book>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse search results for book ISBN: {Isbn}", isbn.SanitizeForLog());
            return new List<Book>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: book ISBN search {Isbn}", isbn.SanitizeForLog());
            return new List<Book>();
        }
    }

    public async Task<List<Book>> GetTrendingAsync(CancellationToken ct = default)
    {
        try
        {
            _logger.LogDebug("Fetching trending books");

            var url = $"{BaseUrl}/trending/daily.json?limit=20";
            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("OpenLibrary trending returned {StatusCode}", response.StatusCode);
                return new List<Book>();
            }

            return ParseTrendingResults(response.Content);
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error fetching trending books");
            return new List<Book>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse response for trending books");
            return new List<Book>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: trending books");
            return new List<Book>();
        }
    }

    public async Task<List<Book>> GetPopularAsync(CancellationToken ct = default)
    {
        return await GetTrendingAsync(ct).ConfigureAwait(false);
    }

    private Book? ParseWork(string json)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            var book = new Book
            {
                Title = root.GetProperty("title").GetString() ?? "Unknown",
                Year = ExtractYear(root),
                Metadata = new BookMetadata
                {
                    Description = GetStringProperty(root, "description"),
                    ForeignBookId = GetStringProperty(root, "key"),
                    OpenLibraryId = GetStringProperty(root, "key"),
                    ReleaseDate = ExtractDate(root),
                    Genres = ExtractGenres(root)
                }
            };

            return book;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing OpenLibrary work JSON");
            return null;
        }
    }

    private List<Book> ParseSearchResults(string json)
    {
        var books = new List<Book>();

        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            if (!root.TryGetProperty("docs", out var docs))
            {
                return books;
            }

            foreach (var item in docs.EnumerateArray())
            {
                try
                {
                    var book = new Book
                    {
                        Title = item.GetProperty("title").GetString() ?? "Unknown",
                        Year = GetIntProperty(item, "first_publish_year") ?? 0,
                        Metadata = new BookMetadata
                        {
                            ForeignBookId = GetStringProperty(item, "key"),
                            OpenLibraryId = GetStringProperty(item, "key"),
                            ReleaseDate = ExtractDateFromYear(GetIntProperty(item, "first_publish_year")),
                            PageCount = GetIntProperty(item, "number_of_pages_median"),
                            Publisher = GetFirstStringFromArray(item, "publisher"),
                            Language = GetFirstStringFromArray(item, "language"),
                            Isbn = GetFirstStringFromArray(item, "isbn"),
                            Genres = GetStringArrayProperty(item, "subject")
                        }
                    };

                    books.Add(book);
                }
                catch (JsonException ex)
                {
                    _logger.LogWarning(ex, "Error parsing individual search result");
                }
            }
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing OpenLibrary search results");
        }

        return books;
    }

    private List<Book> ParseTrendingResults(string json)
    {
        var books = new List<Book>();

        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            if (!root.TryGetProperty("works", out var works))
            {
                return books;
            }

            foreach (var item in works.EnumerateArray())
            {
                try
                {
                    var book = new Book
                    {
                        Title = item.GetProperty("title").GetString() ?? "Unknown",
                        Year = GetIntProperty(item, "first_publish_year") ?? 0,
                        Metadata = new BookMetadata
                        {
                            ForeignBookId = GetStringProperty(item, "key"),
                            OpenLibraryId = GetStringProperty(item, "key"),
                            ReleaseDate = ExtractDateFromYear(GetIntProperty(item, "first_publish_year"))
                        }
                    };

                    books.Add(book);
                }
                catch (JsonException ex)
                {
                    _logger.LogWarning(ex, "Error parsing individual trending result");
                }
            }
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing OpenLibrary trending results");
        }

        return books;
    }

    private static string? GetStringProperty(JsonElement element, string propertyName)
    {
        if (element.TryGetProperty(propertyName, out var property))
        {
            if (property.ValueKind == JsonValueKind.String)
            {
                return property.GetString();
            }
            else if (property.ValueKind == JsonValueKind.Object && property.TryGetProperty("value", out var valueProperty))
            {
                return valueProperty.GetString();
            }
        }

        return null;
    }

    private static int? GetIntProperty(JsonElement element, string propertyName)
    {
        if (element.TryGetProperty(propertyName, out var property) && property.ValueKind == JsonValueKind.Number)
        {
            return property.GetInt32();
        }

        return null;
    }

    private static string? GetFirstStringFromArray(JsonElement element, string propertyName)
    {
        if (element.TryGetProperty(propertyName, out var property) && property.ValueKind == JsonValueKind.Array)
        {
            var first = property.EnumerateArray().FirstOrDefault();
            if (first.ValueKind == JsonValueKind.String)
            {
                return first.GetString();
            }
        }

        return null;
    }

    private static List<string> GetStringArrayProperty(JsonElement element, string propertyName)
    {
        var result = new List<string>();

        if (element.TryGetProperty(propertyName, out var property) && property.ValueKind == JsonValueKind.Array)
        {
            foreach (var item in property.EnumerateArray().Take(10))
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
        var year = GetIntProperty(element, "first_publish_year");
        if (year.HasValue)
        {
            return year.Value;
        }

        var releaseDate = ExtractDate(element);
        return releaseDate?.Year ?? 0;
    }

    private static DateTime? ExtractDate(JsonElement element)
    {
        var dateStr = GetStringProperty(element, "first_publish_date");
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

    private static DateTime? ExtractDateFromYear(int? year)
    {
        if (year.HasValue && year.Value > 0)
        {
            return new DateTime(year.Value, 1, 1);
        }

        return null;
    }

    private static List<string> ExtractGenres(JsonElement element)
    {
        return GetStringArrayProperty(element, "subjects");
    }
}
