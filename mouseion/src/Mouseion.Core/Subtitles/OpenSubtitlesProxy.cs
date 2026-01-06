// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http.Headers;
using System.Text.Json;
using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Subtitles;

public interface IOpenSubtitlesProxy
{
    Task<List<SubtitleSearchResult>> SearchByHashAsync(string movieHash, long fileSize, string? language = null, CancellationToken ct = default);
    Task<List<SubtitleSearchResult>> SearchByImdbAsync(string imdbId, string? language = null, CancellationToken ct = default);
    Task<SubtitleDownloadInfo?> GetDownloadInfoAsync(int fileId, CancellationToken ct = default);
}

public class OpenSubtitlesProxy : IOpenSubtitlesProxy
{
    private readonly HttpClient _httpClient;
    private readonly ILogger<OpenSubtitlesProxy> _logger;
    private const string BaseUrl = "https://api.opensubtitles.com/api/v1";

    public OpenSubtitlesProxy(IHttpClientFactory httpClientFactory, ILogger<OpenSubtitlesProxy> logger)
    {
        _httpClient = httpClientFactory.CreateClient("OpenSubtitles");
        _logger = logger;
    }

    public async Task<List<SubtitleSearchResult>> SearchByHashAsync(string movieHash, long fileSize, string? language = null, CancellationToken ct = default)
    {
        var queryParams = new Dictionary<string, string>
        {
            ["moviehash"] = movieHash,
            ["moviebytesize"] = fileSize.ToString()
        };

        if (!string.IsNullOrEmpty(language))
        {
            queryParams["languages"] = language;
        }

        return await SearchSubtitlesAsync(queryParams, ct).ConfigureAwait(false);
    }

    public async Task<List<SubtitleSearchResult>> SearchByImdbAsync(string imdbId, string? language = null, CancellationToken ct = default)
    {
        var queryParams = new Dictionary<string, string>
        {
            ["imdb_id"] = imdbId.Replace("tt", string.Empty)
        };

        if (!string.IsNullOrEmpty(language))
        {
            queryParams["languages"] = language;
        }

        return await SearchSubtitlesAsync(queryParams, ct).ConfigureAwait(false);
    }

    public async Task<SubtitleDownloadInfo?> GetDownloadInfoAsync(int fileId, CancellationToken ct = default)
    {
        var url = $"{BaseUrl}/download";
        var request = new HttpRequestMessage(HttpMethod.Post, url)
        {
            Content = new StringContent(JsonSerializer.Serialize(new { file_id = fileId }), System.Text.Encoding.UTF8, "application/json")
        };

        var response = await _httpClient.SendAsync(request, ct).ConfigureAwait(false);
        if (!response.IsSuccessStatusCode)
        {
            _logger.LogWarning("OpenSubtitles download info request failed: {StatusCode}", response.StatusCode);
            return null;
        }

        var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        var result = JsonSerializer.Deserialize<OpenSubtitlesDownloadResponse>(json);

        return result?.link != null ? new SubtitleDownloadInfo
        {
            DownloadUrl = result.link,
            FileName = result.file_name ?? "subtitle.srt",
            Remaining = result.remaining ?? 0
        } : null;
    }

    private async Task<List<SubtitleSearchResult>> SearchSubtitlesAsync(Dictionary<string, string> queryParams, CancellationToken ct)
    {
        var queryString = string.Join("&", queryParams.Select(kvp => $"{kvp.Key}={Uri.EscapeDataString(kvp.Value)}"));
        var url = $"{BaseUrl}/subtitles?{queryString}";

        var response = await _httpClient.GetAsync(url, ct).ConfigureAwait(false);
        if (!response.IsSuccessStatusCode)
        {
            _logger.LogWarning("OpenSubtitles search failed: {StatusCode}", response.StatusCode);
            return new List<SubtitleSearchResult>();
        }

        var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        var result = JsonSerializer.Deserialize<OpenSubtitlesSearchResponse>(json);

        return result?.data?.Select(MapToSearchResult).Where(r => r != null).ToList() ?? new List<SubtitleSearchResult>();
    }

    private SubtitleSearchResult? MapToSearchResult(OpenSubtitlesSubtitle subtitle)
    {
        if (subtitle.attributes == null || subtitle.attributes.files == null || subtitle.attributes.files.Count == 0)
        {
            return null;
        }

        var file = subtitle.attributes.files[0];

        return new SubtitleSearchResult
        {
            FileId = file.file_id,
            FileName = file.file_name ?? "subtitle.srt",
            Language = subtitle.attributes.language ?? "en",
            DownloadCount = subtitle.attributes.download_count ?? 0,
            Rating = subtitle.attributes.ratings ?? 0.0f,
            MovieHash = subtitle.attributes.moviehash_match ?? false,
            Uploader = subtitle.attributes.uploader?.name
        };
    }

    private class OpenSubtitlesSearchResponse
    {
        public List<OpenSubtitlesSubtitle>? data { get; set; }
    }

    private class OpenSubtitlesSubtitle
    {
        public SubtitleAttributes? attributes { get; set; }
    }

    private class SubtitleAttributes
    {
        public string? language { get; set; }
        public int? download_count { get; set; }
        public float? ratings { get; set; }
        public bool? moviehash_match { get; set; }
        public UploaderInfo? uploader { get; set; }
        public List<SubtitleFile>? files { get; set; }
    }

    private class SubtitleFile
    {
        public int file_id { get; set; }
        public string? file_name { get; set; }
    }

    private class UploaderInfo
    {
        public string? name { get; set; }
    }

    private class OpenSubtitlesDownloadResponse
    {
        public string? link { get; set; }
        public string? file_name { get; set; }
        public int? remaining { get; set; }
    }
}

public class SubtitleSearchResult
{
    public int FileId { get; set; }
    public string FileName { get; set; } = string.Empty;
    public string Language { get; set; } = string.Empty;
    public int DownloadCount { get; set; }
    public float Rating { get; set; }
    public bool MovieHash { get; set; }
    public string? Uploader { get; set; }
}

public class SubtitleDownloadInfo
{
    public string DownloadUrl { get; set; } = string.Empty;
    public string FileName { get; set; } = string.Empty;
    public int Remaining { get; set; }
}
