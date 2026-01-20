// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http.Json;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Comic.ComicVine;

public interface IComicVineClient
{
    Task<ComicVineVolume?> GetVolumeAsync(int volumeId, CancellationToken ct = default);
    Task<List<ComicVineVolume>> SearchVolumesAsync(string query, int limit = 10, CancellationToken ct = default);
    Task<List<ComicVineIssue>> GetIssuesForVolumeAsync(int volumeId, int limit = 100, int offset = 0, CancellationToken ct = default);
    Task<ComicVineIssue?> GetIssueAsync(int issueId, CancellationToken ct = default);
}

public class ComicVineClient : IComicVineClient
{
    private readonly HttpClient _httpClient;
    private readonly ILogger<ComicVineClient> _logger;
    private readonly string? _apiKey;
    private const string BaseUrl = "https://comicvine.gamespot.com/api";

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
    };

    public ComicVineClient(HttpClient httpClient, ILogger<ComicVineClient> logger)
    {
        _httpClient = httpClient;
        _logger = logger;
        _apiKey = Environment.GetEnvironmentVariable("COMICVINE_API_KEY");

        _httpClient.DefaultRequestHeaders.Add("User-Agent", "Mouseion/1.0");
    }

    public async Task<ComicVineVolume?> GetVolumeAsync(int volumeId, CancellationToken ct = default)
    {
        if (string.IsNullOrEmpty(_apiKey))
        {
            _logger.LogWarning("ComicVine API key not configured");
            return null;
        }

        try
        {
            var url = $"{BaseUrl}/volume/4050-{volumeId}/?api_key={_apiKey}&format=json&field_list=id,name,description,start_year,publisher,image,count_of_issues,site_detail_url";
            var response = await _httpClient.GetFromJsonAsync<ComicVineResponse<ComicVineVolume>>(url, JsonOptions, ct).ConfigureAwait(false);

            if (response?.StatusCode != 1)
            {
                _logger.LogWarning("ComicVine API error: {Error}", response?.Error);
                return null;
            }

            return response.Results;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to get volume {VolumeId} from ComicVine", volumeId);
            return null;
        }
    }

    public async Task<List<ComicVineVolume>> SearchVolumesAsync(string query, int limit = 10, CancellationToken ct = default)
    {
        if (string.IsNullOrEmpty(_apiKey))
        {
            _logger.LogWarning("ComicVine API key not configured");
            return new List<ComicVineVolume>();
        }

        try
        {
            var encodedQuery = Uri.EscapeDataString(query);
            var url = $"{BaseUrl}/search/?api_key={_apiKey}&format=json&resources=volume&query={encodedQuery}&limit={limit}&field_list=id,name,description,start_year,publisher,image,count_of_issues,site_detail_url";
            var response = await _httpClient.GetFromJsonAsync<ComicVineSearchResponse<ComicVineVolume>>(url, JsonOptions, ct).ConfigureAwait(false);

            if (response?.StatusCode != 1)
            {
                _logger.LogWarning("ComicVine search error: {Error}", response?.Error);
                return new List<ComicVineVolume>();
            }

            return response.Results ?? new List<ComicVineVolume>();
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to search ComicVine for '{Query}'", query);
            return new List<ComicVineVolume>();
        }
    }

    public async Task<List<ComicVineIssue>> GetIssuesForVolumeAsync(int volumeId, int limit = 100, int offset = 0, CancellationToken ct = default)
    {
        if (string.IsNullOrEmpty(_apiKey))
        {
            _logger.LogWarning("ComicVine API key not configured");
            return new List<ComicVineIssue>();
        }

        try
        {
            var url = $"{BaseUrl}/issues/?api_key={_apiKey}&format=json&filter=volume:{volumeId}&limit={limit}&offset={offset}&sort=issue_number:asc&field_list=id,name,issue_number,description,cover_date,store_date,image,site_detail_url,volume";
            var response = await _httpClient.GetFromJsonAsync<ComicVineSearchResponse<ComicVineIssue>>(url, JsonOptions, ct).ConfigureAwait(false);

            if (response?.StatusCode != 1)
            {
                _logger.LogWarning("ComicVine issues error: {Error}", response?.Error);
                return new List<ComicVineIssue>();
            }

            return response.Results ?? new List<ComicVineIssue>();
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to get issues for volume {VolumeId} from ComicVine", volumeId);
            return new List<ComicVineIssue>();
        }
    }

    public async Task<ComicVineIssue?> GetIssueAsync(int issueId, CancellationToken ct = default)
    {
        if (string.IsNullOrEmpty(_apiKey))
        {
            _logger.LogWarning("ComicVine API key not configured");
            return null;
        }

        try
        {
            var url = $"{BaseUrl}/issue/4000-{issueId}/?api_key={_apiKey}&format=json&field_list=id,name,issue_number,description,cover_date,store_date,image,site_detail_url,volume,person_credits";
            var response = await _httpClient.GetFromJsonAsync<ComicVineResponse<ComicVineIssue>>(url, JsonOptions, ct).ConfigureAwait(false);

            if (response?.StatusCode != 1)
            {
                _logger.LogWarning("ComicVine API error: {Error}", response?.Error);
                return null;
            }

            return response.Results;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to get issue {IssueId} from ComicVine", issueId);
            return null;
        }
    }
}

public class ComicVineResponse<T>
{
    [JsonPropertyName("error")]
    public string? Error { get; set; }

    [JsonPropertyName("limit")]
    public int Limit { get; set; }

    [JsonPropertyName("offset")]
    public int Offset { get; set; }

    [JsonPropertyName("number_of_page_results")]
    public int NumberOfPageResults { get; set; }

    [JsonPropertyName("number_of_total_results")]
    public int NumberOfTotalResults { get; set; }

    [JsonPropertyName("status_code")]
    public int StatusCode { get; set; }

    [JsonPropertyName("results")]
    public T? Results { get; set; }
}

public class ComicVineSearchResponse<T>
{
    [JsonPropertyName("error")]
    public string? Error { get; set; }

    [JsonPropertyName("limit")]
    public int Limit { get; set; }

    [JsonPropertyName("offset")]
    public int Offset { get; set; }

    [JsonPropertyName("number_of_page_results")]
    public int NumberOfPageResults { get; set; }

    [JsonPropertyName("number_of_total_results")]
    public int NumberOfTotalResults { get; set; }

    [JsonPropertyName("status_code")]
    public int StatusCode { get; set; }

    [JsonPropertyName("results")]
    public List<T>? Results { get; set; }
}

public class ComicVineVolume
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("name")]
    public string? Name { get; set; }

    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("start_year")]
    public string? StartYear { get; set; }

    [JsonPropertyName("count_of_issues")]
    public int? CountOfIssues { get; set; }

    [JsonPropertyName("publisher")]
    public ComicVinePublisher? Publisher { get; set; }

    [JsonPropertyName("image")]
    public ComicVineImage? Image { get; set; }

    [JsonPropertyName("site_detail_url")]
    public string? SiteDetailUrl { get; set; }
}

public class ComicVineIssue
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("name")]
    public string? Name { get; set; }

    [JsonPropertyName("issue_number")]
    public string? IssueNumber { get; set; }

    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("cover_date")]
    public string? CoverDate { get; set; }

    [JsonPropertyName("store_date")]
    public string? StoreDate { get; set; }

    [JsonPropertyName("image")]
    public ComicVineImage? Image { get; set; }

    [JsonPropertyName("site_detail_url")]
    public string? SiteDetailUrl { get; set; }

    [JsonPropertyName("volume")]
    public ComicVineVolumeRef? Volume { get; set; }

    [JsonPropertyName("person_credits")]
    public List<ComicVinePersonCredit>? PersonCredits { get; set; }
}

public class ComicVinePublisher
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("name")]
    public string? Name { get; set; }
}

public class ComicVineImage
{
    [JsonPropertyName("icon_url")]
    public string? IconUrl { get; set; }

    [JsonPropertyName("medium_url")]
    public string? MediumUrl { get; set; }

    [JsonPropertyName("screen_url")]
    public string? ScreenUrl { get; set; }

    [JsonPropertyName("screen_large_url")]
    public string? ScreenLargeUrl { get; set; }

    [JsonPropertyName("small_url")]
    public string? SmallUrl { get; set; }

    [JsonPropertyName("super_url")]
    public string? SuperUrl { get; set; }

    [JsonPropertyName("thumb_url")]
    public string? ThumbUrl { get; set; }

    [JsonPropertyName("tiny_url")]
    public string? TinyUrl { get; set; }

    [JsonPropertyName("original_url")]
    public string? OriginalUrl { get; set; }
}

public class ComicVineVolumeRef
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("name")]
    public string? Name { get; set; }
}

public class ComicVinePersonCredit
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("name")]
    public string? Name { get; set; }

    [JsonPropertyName("role")]
    public string? Role { get; set; }
}
