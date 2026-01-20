// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http.Json;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Manga.MangaDex;

public interface IMangaDexClient
{
    Task<MangaDexManga?> GetMangaAsync(string mangaId, CancellationToken ct = default);
    Task<List<MangaDexManga>> SearchMangaAsync(string query, int limit = 10, CancellationToken ct = default);
    Task<List<MangaDexChapter>> GetChaptersAsync(string mangaId, string? language = "en", int limit = 100, int offset = 0, CancellationToken ct = default);
    Task<MangaDexChapter?> GetChapterAsync(string chapterId, CancellationToken ct = default);
    Task<string?> GetCoverUrlAsync(string mangaId, string? coverId, CancellationToken ct = default);
}

public class MangaDexClient : IMangaDexClient
{
    private const string BaseUrl = "https://api.mangadex.org";
    private readonly ILogger<MangaDexClient> _logger;
    private readonly IHttpClientFactory _httpClientFactory;
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    public MangaDexClient(ILogger<MangaDexClient> logger, IHttpClientFactory httpClientFactory)
    {
        _logger = logger;
        _httpClientFactory = httpClientFactory;
    }

    public async Task<MangaDexManga?> GetMangaAsync(string mangaId, CancellationToken ct = default)
    {
        try
        {
            var client = _httpClientFactory.CreateClient();
            var url = $"{BaseUrl}/manga/{mangaId}?includes[]=cover_art&includes[]=author&includes[]=artist";

            var response = await client.GetFromJsonAsync<MangaDexResponse<MangaDexManga>>(url, JsonOptions, ct).ConfigureAwait(false);
            return response?.Data;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Failed to get manga {MangaId} from MangaDex", mangaId);
            return null;
        }
    }

    public async Task<List<MangaDexManga>> SearchMangaAsync(string query, int limit = 10, CancellationToken ct = default)
    {
        try
        {
            var client = _httpClientFactory.CreateClient();
            var encodedQuery = Uri.EscapeDataString(query);
            var url = $"{BaseUrl}/manga?title={encodedQuery}&limit={limit}&includes[]=cover_art&includes[]=author&includes[]=artist";

            var response = await client.GetFromJsonAsync<MangaDexListResponse<MangaDexManga>>(url, JsonOptions, ct).ConfigureAwait(false);
            return response?.Data ?? new List<MangaDexManga>();
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Failed to search manga with query {Query}", query);
            return new List<MangaDexManga>();
        }
    }

    public async Task<List<MangaDexChapter>> GetChaptersAsync(string mangaId, string? language = "en", int limit = 100, int offset = 0, CancellationToken ct = default)
    {
        try
        {
            var client = _httpClientFactory.CreateClient();
            var url = $"{BaseUrl}/manga/{mangaId}/feed?limit={limit}&offset={offset}&order[chapter]=desc&includes[]=scanlation_group";

            if (!string.IsNullOrEmpty(language))
            {
                url += $"&translatedLanguage[]={language}";
            }

            var response = await client.GetFromJsonAsync<MangaDexListResponse<MangaDexChapter>>(url, JsonOptions, ct).ConfigureAwait(false);
            return response?.Data ?? new List<MangaDexChapter>();
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Failed to get chapters for manga {MangaId}", mangaId);
            return new List<MangaDexChapter>();
        }
    }

    public async Task<MangaDexChapter?> GetChapterAsync(string chapterId, CancellationToken ct = default)
    {
        try
        {
            var client = _httpClientFactory.CreateClient();
            var url = $"{BaseUrl}/chapter/{chapterId}?includes[]=scanlation_group";

            var response = await client.GetFromJsonAsync<MangaDexResponse<MangaDexChapter>>(url, JsonOptions, ct).ConfigureAwait(false);
            return response?.Data;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Failed to get chapter {ChapterId} from MangaDex", chapterId);
            return null;
        }
    }

    public async Task<string?> GetCoverUrlAsync(string mangaId, string? coverId, CancellationToken ct = default)
    {
        if (string.IsNullOrEmpty(coverId))
            return null;

        try
        {
            var client = _httpClientFactory.CreateClient();
            var url = $"{BaseUrl}/cover/{coverId}";

            var response = await client.GetFromJsonAsync<MangaDexResponse<MangaDexCover>>(url, JsonOptions, ct).ConfigureAwait(false);
            var fileName = response?.Data?.Attributes?.FileName;

            if (string.IsNullOrEmpty(fileName))
                return null;

            return $"https://uploads.mangadex.org/covers/{mangaId}/{fileName}";
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Failed to get cover for manga {MangaId}", mangaId);
            return null;
        }
    }
}

public class MangaDexResponse<T>
{
    public string Result { get; set; } = string.Empty;
    public string Response { get; set; } = string.Empty;
    public T? Data { get; set; }
}

public class MangaDexListResponse<T>
{
    public string Result { get; set; } = string.Empty;
    public string Response { get; set; } = string.Empty;
    public List<T> Data { get; set; } = new();
    public int Limit { get; set; }
    public int Offset { get; set; }
    public int Total { get; set; }
}

public class MangaDexManga
{
    public string Id { get; set; } = string.Empty;
    public string Type { get; set; } = string.Empty;
    public MangaDexMangaAttributes Attributes { get; set; } = new();
    public List<MangaDexRelationship> Relationships { get; set; } = new();
}

public class MangaDexMangaAttributes
{
    public Dictionary<string, string> Title { get; set; } = new();
    public List<Dictionary<string, string>> AltTitles { get; set; } = new();
    public Dictionary<string, string> Description { get; set; } = new();
    public bool IsLocked { get; set; }
    public Dictionary<string, string>? Links { get; set; }
    public string OriginalLanguage { get; set; } = string.Empty;
    public string? LastVolume { get; set; }
    public string? LastChapter { get; set; }
    public string? PublicationDemographic { get; set; }
    public string Status { get; set; } = string.Empty;
    public int? Year { get; set; }
    public string ContentRating { get; set; } = string.Empty;
    public List<MangaDexTag> Tags { get; set; } = new();
    public string State { get; set; } = string.Empty;
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
}

public class MangaDexTag
{
    public string Id { get; set; } = string.Empty;
    public string Type { get; set; } = string.Empty;
    public MangaDexTagAttributes Attributes { get; set; } = new();
}

public class MangaDexTagAttributes
{
    public Dictionary<string, string> Name { get; set; } = new();
    public string Group { get; set; } = string.Empty;
}

public class MangaDexChapter
{
    public string Id { get; set; } = string.Empty;
    public string Type { get; set; } = string.Empty;
    public MangaDexChapterAttributes Attributes { get; set; } = new();
    public List<MangaDexRelationship> Relationships { get; set; } = new();
}

public class MangaDexChapterAttributes
{
    public string? Volume { get; set; }
    public string? Chapter { get; set; }
    public string? Title { get; set; }
    public string TranslatedLanguage { get; set; } = string.Empty;
    public string? ExternalUrl { get; set; }
    public DateTime? PublishAt { get; set; }
    public DateTime? ReadableAt { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime UpdatedAt { get; set; }
    public int Pages { get; set; }
}

public class MangaDexRelationship
{
    public string Id { get; set; } = string.Empty;
    public string Type { get; set; } = string.Empty;
    public JsonElement? Attributes { get; set; }
}

public class MangaDexCover
{
    public string Id { get; set; } = string.Empty;
    public string Type { get; set; } = string.Empty;
    public MangaDexCoverAttributes? Attributes { get; set; }
}

public class MangaDexCoverAttributes
{
    public string FileName { get; set; } = string.Empty;
    public string? Description { get; set; }
    public string? Volume { get; set; }
}
