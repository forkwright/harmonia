// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Manga.AniList;

public interface IAniListClient
{
    Task<AniListMedia?> GetMangaByIdAsync(int id, CancellationToken ct = default);
    Task<AniListMedia?> GetMangaByMalIdAsync(int malId, CancellationToken ct = default);
    Task<List<AniListMedia>> SearchMangaAsync(string query, int perPage = 10, CancellationToken ct = default);
}

public class AniListClient : IAniListClient
{
    private const string BaseUrl = "https://graphql.anilist.co";
    private readonly ILogger<AniListClient> _logger;
    private readonly IHttpClientFactory _httpClientFactory;
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    private const string MediaQuery = @"
        query ($id: Int, $idMal: Int) {
            Media(id: $id, idMal: $idMal, type: MANGA) {
                id
                idMal
                title {
                    romaji
                    english
                    native
                }
                description(asHtml: false)
                status
                startDate { year month day }
                endDate { year month day }
                chapters
                volumes
                coverImage { large medium }
                genres
                tags { name rank }
                staff(perPage: 5) {
                    edges {
                        role
                        node { name { full } }
                    }
                }
                averageScore
                popularity
                siteUrl
            }
        }";

    private const string SearchQuery = @"
        query ($search: String, $perPage: Int) {
            Page(perPage: $perPage) {
                media(search: $search, type: MANGA, sort: POPULARITY_DESC) {
                    id
                    idMal
                    title {
                        romaji
                        english
                        native
                    }
                    description(asHtml: false)
                    status
                    startDate { year month day }
                    endDate { year month day }
                    chapters
                    volumes
                    coverImage { large medium }
                    genres
                    tags { name rank }
                    staff(perPage: 5) {
                        edges {
                            role
                            node { name { full } }
                        }
                    }
                    averageScore
                    popularity
                    siteUrl
                }
            }
        }";

    public AniListClient(ILogger<AniListClient> logger, IHttpClientFactory httpClientFactory)
    {
        _logger = logger;
        _httpClientFactory = httpClientFactory;
    }

    public async Task<AniListMedia?> GetMangaByIdAsync(int id, CancellationToken ct = default)
    {
        try
        {
            var response = await ExecuteQueryAsync<AniListMediaResponse>(MediaQuery, new { id }, ct).ConfigureAwait(false);
            return response?.Data?.Media;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Failed to get manga {Id} from AniList", id);
            return null;
        }
    }

    public async Task<AniListMedia?> GetMangaByMalIdAsync(int malId, CancellationToken ct = default)
    {
        try
        {
            var response = await ExecuteQueryAsync<AniListMediaResponse>(MediaQuery, new { idMal = malId }, ct).ConfigureAwait(false);
            return response?.Data?.Media;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Failed to get manga by MAL ID {MalId} from AniList", malId);
            return null;
        }
    }

    public async Task<List<AniListMedia>> SearchMangaAsync(string query, int perPage = 10, CancellationToken ct = default)
    {
        try
        {
            var response = await ExecuteQueryAsync<AniListPageResponse>(SearchQuery, new { search = query, perPage }, ct).ConfigureAwait(false);
            return response?.Data?.Page?.Media ?? new List<AniListMedia>();
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Failed to search manga with query {Query}", query);
            return new List<AniListMedia>();
        }
    }

    private async Task<T?> ExecuteQueryAsync<T>(string query, object variables, CancellationToken ct) where T : class
    {
        var client = _httpClientFactory.CreateClient();
        var requestBody = new { query, variables };
        var content = new StringContent(JsonSerializer.Serialize(requestBody, JsonOptions), Encoding.UTF8, "application/json");

        var response = await client.PostAsync(BaseUrl, content, ct).ConfigureAwait(false);
        response.EnsureSuccessStatusCode();

        return await response.Content.ReadFromJsonAsync<T>(JsonOptions, ct).ConfigureAwait(false);
    }
}

public class AniListMediaResponse
{
    public AniListMediaData? Data { get; set; }
}

public class AniListMediaData
{
    public AniListMedia? Media { get; set; }
}

public class AniListPageResponse
{
    public AniListPageData? Data { get; set; }
}

public class AniListPageData
{
    public AniListPage? Page { get; set; }
}

public class AniListPage
{
    public List<AniListMedia> Media { get; set; } = new();
}

public class AniListMedia
{
    public int Id { get; set; }
    public int? IdMal { get; set; }
    public AniListTitle? Title { get; set; }
    public string? Description { get; set; }
    public string? Status { get; set; }
    public AniListDate? StartDate { get; set; }
    public AniListDate? EndDate { get; set; }
    public int? Chapters { get; set; }
    public int? Volumes { get; set; }
    public AniListCoverImage? CoverImage { get; set; }
    public List<string>? Genres { get; set; }
    public List<AniListTag>? Tags { get; set; }
    public AniListStaffConnection? Staff { get; set; }
    public int? AverageScore { get; set; }
    public int? Popularity { get; set; }
    public string? SiteUrl { get; set; }
}

public class AniListTitle
{
    public string? Romaji { get; set; }
    public string? English { get; set; }
    public string? Native { get; set; }

    public string GetPreferredTitle() => English ?? Romaji ?? Native ?? "Unknown";
}

public class AniListDate
{
    public int? Year { get; set; }
    public int? Month { get; set; }
    public int? Day { get; set; }
}

public class AniListCoverImage
{
    public string? Large { get; set; }
    public string? Medium { get; set; }
}

public class AniListTag
{
    public string? Name { get; set; }
    public int? Rank { get; set; }
}

public class AniListStaffConnection
{
    public List<AniListStaffEdge>? Edges { get; set; }
}

public class AniListStaffEdge
{
    public string? Role { get; set; }
    public AniListStaff? Node { get; set; }
}

public class AniListStaff
{
    public AniListName? Name { get; set; }
}

public class AniListName
{
    public string? Full { get; set; }
}
