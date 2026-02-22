// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Http;

namespace Mouseion.Core.SmartLists.Sources;

/// <summary>
/// Queries MusicBrainz for new releases by tag, area, or date range.
/// No authentication required — uses rate-limited public API.
/// </summary>
public partial class MusicBrainzReleasesProvider : ISmartListSourceProvider
{
    private readonly IHttpClient _httpClient;
    private readonly ILogger<MusicBrainzReleasesProvider> _logger;
    private const string BaseUrl = "https://musicbrainz.org/ws/2";

    public SmartListSource Source => SmartListSource.MusicBrainzReleases;

    public MusicBrainzReleasesProvider(IHttpClient httpClient, ILogger<MusicBrainzReleasesProvider> logger)
    {
        _httpClient = httpClient;
        _logger = logger;
    }

    public async Task<IReadOnlyList<SmartListDiscoveryResult>> DiscoverAsync(SmartList smartList, CancellationToken ct = default)
    {
        var queryParams = ParseQueryParameters(smartList.QueryParametersJson);
        var query = BuildLuceneQuery(queryParams, smartList);
        var url = $"{BaseUrl}/release-group/?query={Uri.EscapeDataString(query)}&fmt=json&limit={smartList.MaxItemsPerRefresh}";

        LogDiscoverRequest(smartList.Name, query);

        var request = new HttpRequest(url)
        {
            AllowAutoRedirect = true
        };

        var response = await _httpClient.GetAsync(request).ConfigureAwait(false);

        if (!response.HasHttpError)
        {
            return ParseMusicBrainzResponse(response.Content);
        }

        LogDiscoverError(smartList.Name, response.StatusCode.ToString());
        return Array.Empty<SmartListDiscoveryResult>();
    }

    private static MusicBrainzQueryParameters ParseQueryParameters(string json)
    {
        try
        {
            return JsonSerializer.Deserialize<MusicBrainzQueryParameters>(json) ?? new MusicBrainzQueryParameters();
        }
        catch
        {
            return new MusicBrainzQueryParameters();
        }
    }

    private static string BuildLuceneQuery(MusicBrainzQueryParameters queryParams, SmartList list)
    {
        var parts = new List<string>();

        if (!string.IsNullOrEmpty(queryParams.Tag))
            parts.Add($"tag:{queryParams.Tag}");

        if (!string.IsNullOrEmpty(queryParams.ReleaseType))
            parts.Add($"primarytype:{queryParams.ReleaseType}");
        else
            parts.Add("primarytype:album");

        if (list.MinYear.HasValue || list.MaxYear.HasValue)
        {
            var from = list.MinYear?.ToString() ?? "*";
            var to = list.MaxYear?.ToString() ?? "*";
            parts.Add($"firstreleasedate:[{from} TO {to}]");
        }
        else
        {
            // Default: last 30 days
            var from = DateTime.UtcNow.AddDays(-30).ToString("yyyy-MM-dd");
            parts.Add($"firstreleasedate:[{from} TO *]");
        }

        if (!string.IsNullOrEmpty(queryParams.Country))
            parts.Add($"country:{queryParams.Country}");

        return string.Join(" AND ", parts);
    }

    private static List<SmartListDiscoveryResult> ParseMusicBrainzResponse(string content)
    {
        var results = new List<SmartListDiscoveryResult>();

        using var doc = JsonDocument.Parse(content);
        if (!doc.RootElement.TryGetProperty("release-groups", out var groups))
            return results;

        foreach (var item in groups.EnumerateArray())
        {
            var title = item.TryGetProperty("title", out var titleProp) ? titleProp.GetString() ?? "" : "";
            var mbid = item.TryGetProperty("id", out var id) ? id.GetString() : null;

            var artist = "";
            if (item.TryGetProperty("artist-credit", out var credits) && credits.ValueKind == JsonValueKind.Array)
            {
                var names = credits.EnumerateArray()
                    .Select(c => c.TryGetProperty("name", out var n) ? n.GetString() ?? "" : "")
                    .Where(n => !string.IsNullOrEmpty(n));
                artist = string.Join(", ", names);
            }

            var year = 0;
            if (item.TryGetProperty("first-release-date", out var frd))
            {
                var dateStr = frd.GetString();
                if (!string.IsNullOrEmpty(dateStr) && dateStr.Length >= 4)
                    int.TryParse(dateStr[..4], out year);
            }

            var result = new SmartListDiscoveryResult
            {
                ExternalId = mbid ?? "",
                Title = $"{artist} — {title}",
                Year = year,
                MusicBrainzId = Guid.TryParse(mbid, out var guid) ? guid : null,
                MetadataJson = item.GetRawText()
            };

            results.Add(result);
        }

        return results;
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "MusicBrainz discover for '{Name}': {Query}")]
    private partial void LogDiscoverRequest(string name, string query);

    [LoggerMessage(Level = LogLevel.Warning, Message = "MusicBrainz discover failed for '{Name}': {StatusCode}")]
    private partial void LogDiscoverError(string name, string statusCode);
}

public class MusicBrainzQueryParameters
{
    public string? Tag { get; set; }
    /// <summary>album, single, ep, compilation</summary>
    public string? ReleaseType { get; set; }
    /// <summary>ISO 3166-1 country code</summary>
    public string? Country { get; set; }
}
