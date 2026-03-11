// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Http;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.SmartLists.Sources;

/// <summary>
/// Queries AniList GraphQL API for anime and manga discovery by genre, score, season, popularity.
/// No authentication required for public queries.
/// </summary>
public partial class AniListDiscoverProvider : ISmartListSourceProvider
{
    private readonly IHttpClient _httpClient;
    private readonly ILogger<AniListDiscoverProvider> _logger;
    private const string ApiUrl = "https://graphql.anilist.co";

    public SmartListSource Source => SmartListSource.AniListDiscover;

    public AniListDiscoverProvider(IHttpClient httpClient, ILogger<AniListDiscoverProvider> logger)
    {
        _httpClient = httpClient;
        _logger = logger;
    }

    public async Task<IReadOnlyList<SmartListDiscoveryResult>> DiscoverAsync(SmartList smartList, CancellationToken ct = default)
    {
        var queryParams = ParseQueryParameters(smartList.QueryParametersJson);
        var aniListType = smartList.MediaType == MediaType.Manga ? "MANGA" : "ANIME";
        var graphqlQuery = BuildGraphqlQuery(aniListType, queryParams, smartList);

        LogDiscoverRequest(smartList.Name, aniListType);

        var requestBody = JsonSerializer.Serialize(new { query = graphqlQuery });
        var request = new HttpRequest(ApiUrl)
        {
            AllowAutoRedirect = true,
            Method = System.Net.Http.HttpMethod.Post
        };
        request.Headers.ContentType = "application/json";
        request.SetContent(requestBody);

        var response = await _httpClient.PostAsync(request).ConfigureAwait(false);

        if (!response.HasHttpError)
        {
            return ParseAniListResponse(response.Content, smartList.MediaType);
        }

        LogDiscoverError(smartList.Name, response.StatusCode.ToString());
        return Array.Empty<SmartListDiscoveryResult>();
    }

    private static AniListQueryParameters ParseQueryParameters(string json)
    {
        try
        {
            return JsonSerializer.Deserialize<AniListQueryParameters>(json) ?? new AniListQueryParameters();
        }
        catch
        {
            return new AniListQueryParameters();
        }
    }

    private static string BuildGraphqlQuery(string type, AniListQueryParameters queryParams, SmartList list)
    {
        var variables = new StringBuilder();

        variables.Append($"type: {type}");
        variables.Append($", sort: {queryParams.Sort ?? "POPULARITY_DESC"}");
        variables.Append($", perPage: {Math.Min(list.MaxItemsPerRefresh, 50)}");

        if (list.MinimumRating.HasValue)
            variables.Append($", averageScore_greater: {list.MinimumRating.Value}");

        if (!string.IsNullOrEmpty(queryParams.Genre))
            variables.Append($", genre: \"{queryParams.Genre}\"");

        if (!string.IsNullOrEmpty(queryParams.Season))
            variables.Append($", season: {queryParams.Season}");

        if (queryParams.SeasonYear > 0)
            variables.Append($", seasonYear: {queryParams.SeasonYear}");

        if (list.MinYear.HasValue)
            variables.Append($", startDate_greater: {list.MinYear.Value}0000");

        if (list.MaxYear.HasValue)
            variables.Append($", startDate_lesser: {list.MaxYear.Value}9999");

        return $$"""
            {
              Page(page: 1) {
                media({{variables}}) {
                  id
                  idMal
                  title { romaji english native }
                  startDate { year }
                  averageScore
                  genres
                  description(asHtml: false)
                  coverImage { large }
                }
              }
            }
            """;
    }

    private static List<SmartListDiscoveryResult> ParseAniListResponse(string content, MediaType mediaType)
    {
        var results = new List<SmartListDiscoveryResult>();

        using var doc = JsonDocument.Parse(content);
        if (!doc.RootElement.TryGetProperty("data", out var data) ||
            !data.TryGetProperty("Page", out var page) ||
            !page.TryGetProperty("media", out var media))
            return results;

        foreach (var item in media.EnumerateArray())
        {
            var title = "";
            if (item.TryGetProperty("title", out var titleObj))
            {
                title = titleObj.TryGetProperty("english", out var en) && en.ValueKind == JsonValueKind.String
                    ? en.GetString() ?? ""
                    : titleObj.TryGetProperty("romaji", out var rom) && rom.ValueKind == JsonValueKind.String
                        ? rom.GetString() ?? "" : "";
            }

            var year = item.TryGetProperty("startDate", out var sd) && sd.TryGetProperty("year", out var y)
                && y.ValueKind == JsonValueKind.Number ? y.GetInt32() : 0;

            var score = item.TryGetProperty("averageScore", out var avg) && avg.ValueKind == JsonValueKind.Number
                ? avg.GetInt32() : (int?)null;

            var aniListId = item.TryGetProperty("id", out var id) ? id.GetInt32() : 0;
            var malId = item.TryGetProperty("idMal", out var mal) && mal.ValueKind == JsonValueKind.Number
                ? mal.GetInt32() : (int?)null;

            var result = new SmartListDiscoveryResult
            {
                ExternalId = aniListId.ToString(),
                Title = title,
                Year = year,
                Rating = score,
                AniListId = aniListId,
                MalId = malId,
                Overview = item.TryGetProperty("description", out var desc) ? desc.GetString() : null,
                PosterUrl = item.TryGetProperty("coverImage", out var ci) && ci.TryGetProperty("large", out var lg)
                    ? lg.GetString() : null,
                Genres = item.TryGetProperty("genres", out var genres) && genres.ValueKind == JsonValueKind.Array
                    ? string.Join(",", genres.EnumerateArray().Select(g => g.GetString())) : null,
                MetadataJson = item.GetRawText()
            };

            results.Add(result);
        }

        return results;
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "AniList discover for '{Name}' (type: {Type})")]
    private partial void LogDiscoverRequest(string name, string type);

    [LoggerMessage(Level = LogLevel.Warning, Message = "AniList discover failed for '{Name}': {StatusCode}")]
    private partial void LogDiscoverError(string name, string statusCode);
}

public class AniListQueryParameters
{
    /// <summary>POPULARITY_DESC, SCORE_DESC, TRENDING_DESC, START_DATE_DESC</summary>
    public string? Sort { get; set; }
    public string? Genre { get; set; }
    /// <summary>WINTER, SPRING, SUMMER, FALL</summary>
    public string? Season { get; set; }
    public int SeasonYear { get; set; }
}
