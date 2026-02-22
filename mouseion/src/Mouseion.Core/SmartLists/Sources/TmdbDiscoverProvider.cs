// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Http;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.SmartLists.Sources;

/// <summary>
/// Queries TMDB Discover API for movies and TV shows matching genre, year, rating, and keyword filters.
/// </summary>
public partial class TmdbDiscoverProvider : ISmartListSourceProvider
{
    private readonly IHttpClient _httpClient;
    private readonly ILogger<TmdbDiscoverProvider> _logger;
    private const string BaseUrl = "https://api.themoviedb.org/3";

    public SmartListSource Source => SmartListSource.TmdbDiscover;

    public TmdbDiscoverProvider(IHttpClient httpClient, ILogger<TmdbDiscoverProvider> logger)
    {
        _httpClient = httpClient;
        _logger = logger;
    }

    public async Task<IReadOnlyList<SmartListDiscoveryResult>> DiscoverAsync(SmartList smartList, CancellationToken ct = default)
    {
        var queryParams = ParseQueryParameters(smartList.QueryParametersJson);
        var mediaEndpoint = smartList.MediaType == MediaType.TV ? "tv" : "movie";
        var url = BuildDiscoverUrl(mediaEndpoint, queryParams, smartList);

        LogDiscoverRequest(smartList.Name, url);

        var request = new HttpRequest(url) { AllowAutoRedirect = true };
        var response = await _httpClient.GetAsync(request).ConfigureAwait(false);

        if (!response.HasHttpError)
        {
            return ParseDiscoverResponse(response.Content, smartList.MediaType);
        }

        LogDiscoverError(smartList.Name, response.StatusCode.ToString());
        return Array.Empty<SmartListDiscoveryResult>();
    }

    private static TmdbQueryParameters ParseQueryParameters(string json)
    {
        try
        {
            return JsonSerializer.Deserialize<TmdbQueryParameters>(json) ?? new TmdbQueryParameters();
        }
        catch
        {
            return new TmdbQueryParameters();
        }
    }

    private static string BuildDiscoverUrl(string endpoint, TmdbQueryParameters query, SmartList list)
    {
        var parts = new List<string>
        {
            $"{BaseUrl}/discover/{endpoint}?sort_by={query.SortBy ?? "popularity.desc"}"
        };

        if (!string.IsNullOrEmpty(query.WithGenres))
            parts.Add($"with_genres={query.WithGenres}");

        if (!string.IsNullOrEmpty(query.WithKeywords))
            parts.Add($"with_keywords={query.WithKeywords}");

        if (list.MinYear.HasValue)
        {
            var dateField = endpoint == "tv" ? "first_air_date.gte" : "primary_release_date.gte";
            parts.Add($"{dateField}={list.MinYear.Value}-01-01");
        }

        if (list.MaxYear.HasValue)
        {
            var dateField = endpoint == "tv" ? "first_air_date.lte" : "primary_release_date.lte";
            parts.Add($"{dateField}={list.MaxYear.Value}-12-31");
        }

        if (list.MinimumRating.HasValue)
            parts.Add($"vote_average.gte={list.MinimumRating.Value / 10.0:F1}");

        if (!string.IsNullOrEmpty(list.ExcludeGenres))
            parts.Add($"without_genres={list.ExcludeGenres}");

        if (!string.IsNullOrEmpty(list.Language))
            parts.Add($"with_original_language={list.Language}");

        if (query.VoteCountMinimum > 0)
            parts.Add($"vote_count.gte={query.VoteCountMinimum}");

        parts.Add($"page={query.Page}");

        return string.Join("&", parts);
    }

    private static List<SmartListDiscoveryResult> ParseDiscoverResponse(string content, MediaType mediaType)
    {
        var results = new List<SmartListDiscoveryResult>();

        using var doc = JsonDocument.Parse(content);
        var root = doc.RootElement;

        if (!root.TryGetProperty("results", out var resultsArray))
            return results;

        foreach (var item in resultsArray.EnumerateArray())
        {
            var title = mediaType == MediaType.TV
                ? item.TryGetProperty("name", out var name) ? name.GetString() ?? "" : ""
                : item.TryGetProperty("title", out var titleProp) ? titleProp.GetString() ?? "" : "";

            var dateStr = mediaType == MediaType.TV
                ? item.TryGetProperty("first_air_date", out var fad) ? fad.GetString() : null
                : item.TryGetProperty("release_date", out var rd) ? rd.GetString() : null;

            var year = 0;
            if (!string.IsNullOrEmpty(dateStr) && dateStr.Length >= 4)
                int.TryParse(dateStr[..4], out year);

            var voteAvg = item.TryGetProperty("vote_average", out var va) ? va.GetDouble() : 0;

            var result = new SmartListDiscoveryResult
            {
                ExternalId = item.TryGetProperty("id", out var id) ? id.GetInt32().ToString() : "",
                Title = title,
                Year = year,
                Rating = (int)(voteAvg * 10), // Normalize 0-10 to 0-100
                TmdbId = item.TryGetProperty("id", out var tmdbId) ? tmdbId.GetInt32() : null,
                Overview = item.TryGetProperty("overview", out var ov) ? ov.GetString() : null,
                PosterUrl = item.TryGetProperty("poster_path", out var pp) && pp.GetString() is string path
                    ? $"https://image.tmdb.org/t/p/w500{path}" : null,
                MetadataJson = item.GetRawText()
            };

            if (item.TryGetProperty("genre_ids", out var genres))
            {
                var genreIds = genres.EnumerateArray().Select(g => g.GetInt32().ToString());
                result.Genres = string.Join(",", genreIds);
            }

            results.Add(result);
        }

        return results;
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "TMDB discover request for '{Name}': {Url}")]
    private partial void LogDiscoverRequest(string name, string url);

    [LoggerMessage(Level = LogLevel.Warning, Message = "TMDB discover failed for '{Name}': {StatusCode}")]
    private partial void LogDiscoverError(string name, string statusCode);
}

public class TmdbQueryParameters
{
    public string? SortBy { get; set; }
    public string? WithGenres { get; set; }
    public string? WithKeywords { get; set; }
    public int VoteCountMinimum { get; set; }
    public int Page { get; set; } = 1;
}
