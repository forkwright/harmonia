// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Http;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.SmartLists.Sources;

/// <summary>
/// Queries Trakt public endpoints for trending, popular, anticipated, and box office items.
/// Does NOT require user OAuth — these are public discovery lists.
/// </summary>
public partial class TraktPublicProvider : ISmartListSourceProvider
{
    private readonly IHttpClient _httpClient;
    private readonly ILogger<TraktPublicProvider> _logger;
    private const string BaseUrl = "https://api.trakt.tv";

    public SmartListSource Source => SmartListSource.TraktPublic;

    public TraktPublicProvider(IHttpClient httpClient, ILogger<TraktPublicProvider> logger)
    {
        _httpClient = httpClient;
        _logger = logger;
    }

    public async Task<IReadOnlyList<SmartListDiscoveryResult>> DiscoverAsync(SmartList smartList, CancellationToken ct = default)
    {
        var queryParams = ParseQueryParameters(smartList.QueryParametersJson);
        var mediaEndpoint = smartList.MediaType == MediaType.TV ? "shows" : "movies";
        var listType = queryParams.ListType ?? "popular";
        var url = $"{BaseUrl}/{mediaEndpoint}/{listType}?extended=full&limit={smartList.MaxItemsPerRefresh}";

        if (!string.IsNullOrEmpty(queryParams.Genres))
            url += $"&genres={queryParams.Genres}";

        if (!string.IsNullOrEmpty(queryParams.Years))
            url += $"&years={queryParams.Years}";

        if (!string.IsNullOrEmpty(queryParams.Languages))
            url += $"&languages={queryParams.Languages}";

        LogDiscoverRequest(smartList.Name, listType, url);

        var request = new HttpRequest(url) { AllowAutoRedirect = true };
        var response = await _httpClient.GetAsync(request).ConfigureAwait(false);

        if (!response.HasHttpError)
        {
            return ParseTraktResponse(response.Content, smartList.MediaType, listType);
        }

        LogDiscoverError(smartList.Name, response.StatusCode.ToString());
        return Array.Empty<SmartListDiscoveryResult>();
    }

    private static TraktQueryParameters ParseQueryParameters(string json)
    {
        try
        {
            return JsonSerializer.Deserialize<TraktQueryParameters>(json) ?? new TraktQueryParameters();
        }
        catch
        {
            return new TraktQueryParameters();
        }
    }

    private static List<SmartListDiscoveryResult> ParseTraktResponse(string content, MediaType mediaType, string listType)
    {
        var results = new List<SmartListDiscoveryResult>();

        using var doc = JsonDocument.Parse(content);
        var root = doc.RootElement;

        foreach (var item in root.EnumerateArray())
        {
            // Trending/anticipated responses wrap the media item; popular returns it directly
            var mediaObj = item.TryGetProperty(mediaType == MediaType.TV ? "show" : "movie", out var wrapped)
                ? wrapped
                : item;

            var title = mediaObj.TryGetProperty("title", out var titleProp) ? titleProp.GetString() ?? "" : "";
            var year = mediaObj.TryGetProperty("year", out var yearProp) && yearProp.ValueKind == JsonValueKind.Number
                ? yearProp.GetInt32() : 0;

            var rating = mediaObj.TryGetProperty("rating", out var ratingProp) && ratingProp.ValueKind == JsonValueKind.Number
                ? (int)(ratingProp.GetDouble() * 10) : (int?)null;

            var ids = mediaObj.TryGetProperty("ids", out var idsProp) ? idsProp : default;

            var result = new SmartListDiscoveryResult
            {
                ExternalId = ids.ValueKind != JsonValueKind.Undefined && ids.TryGetProperty("trakt", out var trakt)
                    ? trakt.GetInt32().ToString() : "",
                Title = title,
                Year = year,
                Rating = rating,
                TmdbId = ids.ValueKind != JsonValueKind.Undefined && ids.TryGetProperty("tmdb", out var tmdb)
                    && tmdb.ValueKind == JsonValueKind.Number ? tmdb.GetInt32() : null,
                ImdbId = ids.ValueKind != JsonValueKind.Undefined && ids.TryGetProperty("imdb", out var imdb)
                    ? imdb.GetString() : null,
                TvdbId = ids.ValueKind != JsonValueKind.Undefined && ids.TryGetProperty("tvdb", out var tvdb)
                    && tvdb.ValueKind == JsonValueKind.Number ? tvdb.GetInt32() : null,
                Overview = mediaObj.TryGetProperty("overview", out var ov) ? ov.GetString() : null,
                Genres = mediaObj.TryGetProperty("genres", out var genres) && genres.ValueKind == JsonValueKind.Array
                    ? string.Join(",", genres.EnumerateArray().Select(g => g.GetString())) : null,
                MetadataJson = mediaObj.GetRawText()
            };

            results.Add(result);
        }

        return results;
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Trakt {ListType} discover for '{Name}': {Url}")]
    private partial void LogDiscoverRequest(string name, string listType, string url);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Trakt discover failed for '{Name}': {StatusCode}")]
    private partial void LogDiscoverError(string name, string statusCode);
}

public class TraktQueryParameters
{
    /// <summary>trending, popular, anticipated, boxoffice</summary>
    public string? ListType { get; set; }
    public string? Genres { get; set; }
    public string? Years { get; set; }
    public string? Languages { get; set; }
}
