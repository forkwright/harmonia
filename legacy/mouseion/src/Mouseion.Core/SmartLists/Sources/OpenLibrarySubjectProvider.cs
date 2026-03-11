// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Http;

namespace Mouseion.Core.SmartLists.Sources;

/// <summary>
/// Queries OpenLibrary Subjects API for books by subject/topic.
/// No authentication required — fully public API.
/// </summary>
public partial class OpenLibrarySubjectProvider : ISmartListSourceProvider
{
    private readonly IHttpClient _httpClient;
    private readonly ILogger<OpenLibrarySubjectProvider> _logger;
    private const string BaseUrl = "https://openlibrary.org";

    public SmartListSource Source => SmartListSource.OpenLibrarySubject;

    public OpenLibrarySubjectProvider(IHttpClient httpClient, ILogger<OpenLibrarySubjectProvider> logger)
    {
        _httpClient = httpClient;
        _logger = logger;
    }

    public async Task<IReadOnlyList<SmartListDiscoveryResult>> DiscoverAsync(SmartList smartList, CancellationToken ct = default)
    {
        var queryParams = ParseQueryParameters(smartList.QueryParametersJson);
        var subject = queryParams.Subject ?? "science_fiction";
        var url = $"{BaseUrl}/subjects/{Uri.EscapeDataString(subject)}.json?limit={smartList.MaxItemsPerRefresh}";

        if (queryParams.PublishedInRange is not null)
            url += $"&published_in={queryParams.PublishedInRange}";

        LogDiscoverRequest(smartList.Name, subject);

        var request = new HttpRequest(url) { AllowAutoRedirect = true };
        var response = await _httpClient.GetAsync(request).ConfigureAwait(false);

        if (!response.HasHttpError)
        {
            return ParseSubjectResponse(response.Content);
        }

        LogDiscoverError(smartList.Name, response.StatusCode.ToString());
        return Array.Empty<SmartListDiscoveryResult>();
    }

    private static OpenLibraryQueryParameters ParseQueryParameters(string json)
    {
        try
        {
            return JsonSerializer.Deserialize<OpenLibraryQueryParameters>(json) ?? new OpenLibraryQueryParameters();
        }
        catch
        {
            return new OpenLibraryQueryParameters();
        }
    }

    private static List<SmartListDiscoveryResult> ParseSubjectResponse(string content)
    {
        var results = new List<SmartListDiscoveryResult>();

        using var doc = JsonDocument.Parse(content);
        if (!doc.RootElement.TryGetProperty("works", out var works))
            return results;

        foreach (var item in works.EnumerateArray())
        {
            var title = item.TryGetProperty("title", out var titleProp) ? titleProp.GetString() ?? "" : "";

            var key = item.TryGetProperty("key", out var keyProp) ? keyProp.GetString() ?? "" : "";
            // key format: /works/OL12345W — extract the OL ID
            var externalId = key.Split('/').LastOrDefault() ?? key;

            var year = 0;
            if (item.TryGetProperty("first_publish_year", out var fpy) && fpy.ValueKind == JsonValueKind.Number)
                year = fpy.GetInt32();

            var authors = "";
            if (item.TryGetProperty("authors", out var authorsArr) && authorsArr.ValueKind == JsonValueKind.Array)
            {
                var names = authorsArr.EnumerateArray()
                    .Select(a => a.TryGetProperty("name", out var n) ? n.GetString() ?? "" : "")
                    .Where(n => !string.IsNullOrEmpty(n));
                authors = string.Join(", ", names);
            }

            var isbn = "";
            if (item.TryGetProperty("availability", out var avail) && avail.TryGetProperty("isbn", out var isbnProp))
                isbn = isbnProp.GetString() ?? "";

            var result = new SmartListDiscoveryResult
            {
                ExternalId = externalId,
                Title = string.IsNullOrEmpty(authors) ? title : $"{title} — {authors}",
                Year = year,
                Isbn = string.IsNullOrEmpty(isbn) ? null : isbn,
                PosterUrl = item.TryGetProperty("cover_id", out var cid) && cid.ValueKind == JsonValueKind.Number
                    ? $"https://covers.openlibrary.org/b/id/{cid.GetInt64()}-M.jpg" : null,
                MetadataJson = item.GetRawText()
            };

            // Extract subjects as genres
            if (item.TryGetProperty("subject", out var subjects) && subjects.ValueKind == JsonValueKind.Array)
            {
                result.Genres = string.Join(",", subjects.EnumerateArray()
                    .Take(5)
                    .Select(s => s.GetString()));
            }

            results.Add(result);
        }

        return results;
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "OpenLibrary discover for '{Name}' (subject: {Subject})")]
    private partial void LogDiscoverRequest(string name, string subject);

    [LoggerMessage(Level = LogLevel.Warning, Message = "OpenLibrary discover failed for '{Name}': {StatusCode}")]
    private partial void LogDiscoverError(string name, string statusCode);
}

public class OpenLibraryQueryParameters
{
    /// <summary>Subject slug (e.g., "science_fiction", "fantasy", "history")</summary>
    public string? Subject { get; set; }
    /// <summary>Year range (e.g., "2020-2025")</summary>
    public string? PublishedInRange { get; set; }
}
