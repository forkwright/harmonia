// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Web;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Http;
using Mouseion.Core.Music;

namespace Mouseion.Core.Indexers.Gazelle;

public class GazelleIndexer
{
    private readonly IHttpClient _httpClient;
    private readonly GazelleSettings _settings;
    private readonly GazelleParser _parser;
    private readonly ILogger<GazelleIndexer> _logger;

    public string Name => "Gazelle";
    public bool Enabled => _settings.Enabled;

    public GazelleIndexer(
        IHttpClient httpClient,
        GazelleSettings settings,
        GazelleParser parser,
        ILogger<GazelleIndexer> logger)
    {
        _httpClient = httpClient;
        _settings = settings;
        _parser = parser;
        _logger = logger;
    }

    public async Task<List<GazelleRelease>> SearchAsync(string artistName, string? albumName = null, CancellationToken ct = default)
    {
        try
        {
            _logger.LogDebug("Searching Gazelle for artist: {Artist}, album: {Album}", artistName, albumName ?? "any");

            var searchUrl = BuildSearchUrl(artistName, albumName);
            var request = new HttpRequestBuilder(searchUrl)
                .SetHeader("User-Agent", "Mouseion/1.0")
                .SetHeader("Authorization", _settings.ApiKey)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("Gazelle returned {StatusCode}", response.StatusCode);
                return new List<GazelleRelease>();
            }

            var releases = _parser.ParseSearchResponse(response.Content);

            // Apply filters
            releases = ApplyFilters(releases);

            _logger.LogInformation("Found {Count} releases for {Artist}", releases.Count, artistName);
            return releases;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error searching Gazelle for: {Artist}", artistName);
            return new List<GazelleRelease>();
        }
    }

    public List<GazelleRelease> Search(string artistName, string? albumName = null)
    {
        try
        {
            _logger.LogDebug("Searching Gazelle for artist: {Artist}, album: {Album}", artistName, albumName ?? "any");

            var searchUrl = BuildSearchUrl(artistName, albumName);
            var request = new HttpRequestBuilder(searchUrl)
                .SetHeader("User-Agent", "Mouseion/1.0")
                .SetHeader("Authorization", _settings.ApiKey)
                .Build();

            var response = _httpClient.Get(request);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("Gazelle returned {StatusCode}", response.StatusCode);
                return new List<GazelleRelease>();
            }

            var releases = _parser.ParseSearchResponse(response.Content);

            // Apply filters
            releases = ApplyFilters(releases);

            _logger.LogInformation("Found {Count} releases for {Artist}", releases.Count, artistName);
            return releases;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error searching Gazelle for: {Artist}", artistName);
            return new List<GazelleRelease>();
        }
    }

    private string BuildSearchUrl(string artistName, string? albumName)
    {
        var queryParams = new Dictionary<string, string>
        {
            ["artistname"] = artistName
        };

        if (!string.IsNullOrWhiteSpace(albumName))
        {
            queryParams["groupname"] = albumName;
        }

        var queryString = string.Join("&", queryParams.Select(kvp =>
            $"{HttpUtility.UrlEncode(kvp.Key)}={HttpUtility.UrlEncode(kvp.Value)}"));

        return $"{_settings.BaseUrl.TrimEnd('/')}/ajax.php?action=browse&{queryString}";
    }

    private List<GazelleRelease> ApplyFilters(List<GazelleRelease> releases)
    {
        var filtered = releases.AsEnumerable();

        // Minimum seeders
        filtered = filtered.Where(r => r.Seeders >= _settings.MinimumSeeders);

        // Prefer FLAC
        if (_settings.PreferFlac)
        {
            var flacReleases = filtered.Where(r => r.Format.Equals("FLAC", StringComparison.OrdinalIgnoreCase)).ToList();
            if (flacReleases.Count > 0)
            {
                filtered = flacReleases;
            }
        }

        // Require log score
        if (_settings.PreferLogScored)
        {
            var scoredReleases = filtered.Where(r => r.HasLog && r.LogScore >= _settings.MinimumLogScore).ToList();
            if (scoredReleases.Count > 0)
            {
                filtered = scoredReleases;
            }
        }

        // Require cue
        if (_settings.RequireCue)
        {
            filtered = filtered.Where(r => r.HasCue);
        }

        return filtered.ToList();
    }
}
