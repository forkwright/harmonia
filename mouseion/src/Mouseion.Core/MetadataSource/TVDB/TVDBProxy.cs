// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.Logging;
using Mouseion.Core.TV;

namespace Mouseion.Core.MetadataSource.TVDB;

public interface ITVDBProxy
{
    Task<Series?> GetSeriesByTvdbIdAsync(int tvdbId, CancellationToken ct = default);
    Task<List<Episode>> GetEpisodesBySeriesIdAsync(int tvdbId, CancellationToken ct = default);
    Task<List<Series>> SearchSeriesAsync(string query, CancellationToken ct = default);
}

public class TVDBProxy : ITVDBProxy
{
    private static readonly TimeSpan CacheExpiry = TimeSpan.FromMinutes(15);

    private readonly ITVDBClient _client;
    private readonly IMemoryCache _cache;
    private readonly ILogger<TVDBProxy> _logger;

    public TVDBProxy(
        ITVDBClient client,
        IMemoryCache cache,
        ILogger<TVDBProxy> logger)
    {
        _client = client;
        _cache = cache;
        _logger = logger;
    }

    public async Task<Series?> GetSeriesByTvdbIdAsync(int tvdbId, CancellationToken ct = default)
    {
        var cacheKey = $"tvdb_series_{tvdbId}";

        if (_cache.TryGetValue(cacheKey, out Series? cached))
        {
            _logger.LogDebug("Cache hit for TVDB series: {TvdbId}", tvdbId);
            return cached;
        }

        _logger.LogInformation("Fetching series from TVDB: {TvdbId}", tvdbId);

        try
        {
            var content = await _client.GetAsync($"series/{tvdbId}/extended", ct).ConfigureAwait(false);
            if (string.IsNullOrEmpty(content))
            {
                return null;
            }

            var series = ParseSeries(content);
            if (series != null)
            {
                _cache.Set(cacheKey, series, CacheExpiry);
            }

            return series;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse TVDB series response: {TvdbId}", tvdbId);
            return null;
        }
    }

    public async Task<List<Episode>> GetEpisodesBySeriesIdAsync(int tvdbId, CancellationToken ct = default)
    {
        var cacheKey = $"tvdb_episodes_{tvdbId}";

        if (_cache.TryGetValue(cacheKey, out List<Episode>? cached))
        {
            _logger.LogDebug("Cache hit for TVDB episodes: {TvdbId}", tvdbId);
            return cached ?? new List<Episode>();
        }

        _logger.LogInformation("Fetching episodes from TVDB for series: {TvdbId}", tvdbId);

        var allEpisodes = new List<Episode>();
        var page = 0;

        try
        {
            while (true)
            {
                var content = await _client.GetAsync($"series/{tvdbId}/episodes/default?page={page}", ct).ConfigureAwait(false);
                if (string.IsNullOrEmpty(content))
                {
                    break;
                }

                var (episodes, hasMore) = ParseEpisodes(content);
                allEpisodes.AddRange(episodes);

                if (!hasMore)
                {
                    break;
                }

                page++;

                // Safety limit to prevent infinite loops
                if (page > 50)
                {
                    _logger.LogWarning("TVDB episode pagination exceeded limit for series: {TvdbId}", tvdbId);
                    break;
                }
            }

            _cache.Set(cacheKey, allEpisodes, CacheExpiry);
            _logger.LogDebug("Retrieved {Count} episodes for series {TvdbId}", allEpisodes.Count, tvdbId);

            return allEpisodes;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse TVDB episodes response for series: {TvdbId}", tvdbId);
            return new List<Episode>();
        }
    }

    public async Task<List<Series>> SearchSeriesAsync(string query, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(query))
        {
            return new List<Series>();
        }

        var cacheKey = $"tvdb_search_{query.ToLowerInvariant()}";

        if (_cache.TryGetValue(cacheKey, out List<Series>? cached))
        {
            _logger.LogDebug("Cache hit for TVDB search: {Query}", query);
            return cached ?? new List<Series>();
        }

        _logger.LogInformation("Searching TVDB for: {Query}", query);

        try
        {
            var encodedQuery = Uri.EscapeDataString(query);
            var content = await _client.GetAsync($"search?query={encodedQuery}&type=series", ct).ConfigureAwait(false);

            if (string.IsNullOrEmpty(content))
            {
                return new List<Series>();
            }

            var results = ParseSearchResults(content);
            _cache.Set(cacheKey, results, CacheExpiry);

            _logger.LogDebug("TVDB search returned {Count} results for: {Query}", results.Count, query);
            return results;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse TVDB search response for: {Query}", query);
            return new List<Series>();
        }
    }

    private Series? ParseSeries(string json)
    {
        using var doc = JsonDocument.Parse(json);

        if (!doc.RootElement.TryGetProperty("data", out var data))
        {
            _logger.LogWarning("TVDB response missing 'data' property");
            return null;
        }

        return MapSeries(data);
    }

    private Series MapSeries(JsonElement element)
    {
        var series = new Series
        {
            TvdbId = JsonHelpers.GetIntProperty(element, "id"),
            Title = JsonHelpers.GetStringProperty(element, "name") ?? "Unknown",
            Overview = JsonHelpers.GetStringProperty(element, "overview"),
            Status = MapStatus(JsonHelpers.GetStringProperty(element, "status")?.ToLowerInvariant()),
            Network = GetOriginalNetwork(element),
            Genres = GetGenres(element),
            Images = GetArtwork(element)
        };

        // Extract year from firstAired
        var firstAired = JsonHelpers.GetStringProperty(element, "firstAired");
        if (DateTime.TryParse(firstAired, out var date))
        {
            series.Year = date.Year;
            series.FirstAired = date;
        }

        // Extract runtime from averageRuntime
        series.Runtime = JsonHelpers.GetIntProperty(element, "averageRuntime");

        // Try to get aliases for sort title
        if (element.TryGetProperty("aliases", out var aliases) && aliases.ValueKind == JsonValueKind.Array)
        {
            foreach (var alias in aliases.EnumerateArray())
            {
                if (JsonHelpers.GetStringProperty(alias, "language") == "eng")
                {
                    series.SortTitle = JsonHelpers.GetStringProperty(alias, "name");
                    break;
                }
            }
        }

        // Get remote IDs (IMDB, TMDB)
        if (element.TryGetProperty("remoteIds", out var remoteIds) && remoteIds.ValueKind == JsonValueKind.Array)
        {
            foreach (var remote in remoteIds.EnumerateArray())
            {
                var sourceName = JsonHelpers.GetStringProperty(remote, "sourceName")?.ToLowerInvariant();
                var id = JsonHelpers.GetStringProperty(remote, "id");

                if (sourceName == "imdb" && !string.IsNullOrEmpty(id))
                {
                    series.ImdbId = id;
                }
                else if (sourceName == "themoviedb" && int.TryParse(id, out var tmdbId))
                {
                    series.TmdbId = tmdbId;
                }
            }
        }

        return series;
    }

    private (List<Episode> Episodes, bool HasMore) ParseEpisodes(string json)
    {
        var episodes = new List<Episode>();

        using var doc = JsonDocument.Parse(json);

        if (!doc.RootElement.TryGetProperty("data", out var data))
        {
            return (episodes, false);
        }

        if (!data.TryGetProperty("episodes", out var episodesArray) || episodesArray.ValueKind != JsonValueKind.Array)
        {
            return (episodes, false);
        }

        foreach (var ep in episodesArray.EnumerateArray())
        {
            var episode = new Episode
            {
                SeasonNumber = JsonHelpers.GetIntProperty(ep, "seasonNumber") ?? 0,
                EpisodeNumber = JsonHelpers.GetIntProperty(ep, "number") ?? 0,
                AbsoluteEpisodeNumber = JsonHelpers.GetIntProperty(ep, "absoluteNumber"),
                Title = JsonHelpers.GetStringProperty(ep, "name"),
                Overview = JsonHelpers.GetStringProperty(ep, "overview")
            };

            var aired = JsonHelpers.GetStringProperty(ep, "aired");
            if (DateTime.TryParse(aired, out var airDate))
            {
                episode.AirDate = airDate;
                episode.AirDateUtc = DateTime.SpecifyKind(airDate, DateTimeKind.Utc);
            }

            // Only include actual episodes (skip specials unless season 0)
            if (episode.EpisodeNumber > 0)
            {
                episodes.Add(episode);
            }
        }

        // Check pagination
        var hasMore = false;
        if (doc.RootElement.TryGetProperty("links", out var links))
        {
            var next = JsonHelpers.GetStringProperty(links, "next");
            hasMore = !string.IsNullOrEmpty(next);
        }

        return (episodes, hasMore);
    }

    private List<Series> ParseSearchResults(string json)
    {
        var results = new List<Series>();

        using var doc = JsonDocument.Parse(json);

        if (!doc.RootElement.TryGetProperty("data", out var data) || data.ValueKind != JsonValueKind.Array)
        {
            return results;
        }

        foreach (var item in data.EnumerateArray())
        {
            // Search results have a different structure, type field indicates series
            var type = JsonHelpers.GetStringProperty(item, "type");
            if (type != "series")
            {
                continue;
            }

            var series = new Series
            {
                Title = JsonHelpers.GetStringProperty(item, "name") ?? "Unknown",
                Overview = JsonHelpers.GetStringProperty(item, "overview"),
                Status = MapStatus(JsonHelpers.GetStringProperty(item, "status")?.ToLowerInvariant()),
                Network = JsonHelpers.GetStringProperty(item, "network")
            };

            // TVDB ID from tvdb_id field in search
            var tvdbIdStr = JsonHelpers.GetStringProperty(item, "tvdb_id");
            if (int.TryParse(tvdbIdStr, out var tvdbId))
            {
                series.TvdbId = tvdbId;
            }

            // Year from year field
            var yearStr = JsonHelpers.GetStringProperty(item, "year");
            if (int.TryParse(yearStr, out var year))
            {
                series.Year = year;
            }

            // First aired
            var firstAired = JsonHelpers.GetStringProperty(item, "first_air_time");
            if (DateTime.TryParse(firstAired, out var date))
            {
                series.FirstAired = date;
                if (series.Year == 0)
                {
                    series.Year = date.Year;
                }
            }

            // Image
            var imageUrl = JsonHelpers.GetStringProperty(item, "image_url");
            if (!string.IsNullOrEmpty(imageUrl))
            {
                series.Images.Add(imageUrl);
            }

            // Remote IDs
            if (item.TryGetProperty("remote_ids", out var remoteIds) && remoteIds.ValueKind == JsonValueKind.Array)
            {
                foreach (var remote in remoteIds.EnumerateArray())
                {
                    var sourceName = JsonHelpers.GetStringProperty(remote, "sourceName")?.ToLowerInvariant();
                    var id = JsonHelpers.GetStringProperty(remote, "id");

                    if (sourceName == "imdb" && !string.IsNullOrEmpty(id))
                    {
                        series.ImdbId = id;
                    }
                }
            }

            results.Add(series);
        }

        return results;
    }

    private static string MapStatus(string? status)
    {
        return status switch
        {
            "continuing" => "Continuing",
            "ended" => "Ended",
            "upcoming" => "Upcoming",
            _ => "Continuing"
        };
    }

    private static string? GetOriginalNetwork(JsonElement element)
    {
        if (element.TryGetProperty("originalNetwork", out var network) && network.ValueKind == JsonValueKind.Object)
        {
            return JsonHelpers.GetStringProperty(network, "name");
        }

        return JsonHelpers.GetStringProperty(element, "network");
    }

    private static List<string> GetGenres(JsonElement element)
    {
        var genres = new List<string>();

        if (element.TryGetProperty("genres", out var genresArray) && genresArray.ValueKind == JsonValueKind.Array)
        {
            foreach (var genre in genresArray.EnumerateArray())
            {
                var name = JsonHelpers.GetStringProperty(genre, "name");
                if (!string.IsNullOrEmpty(name))
                {
                    genres.Add(name);
                }
            }
        }

        return genres;
    }

    private static List<string> GetArtwork(JsonElement element)
    {
        var images = new List<string>();

        if (element.TryGetProperty("artworks", out var artworks) && artworks.ValueKind == JsonValueKind.Array)
        {
            foreach (var art in artworks.EnumerateArray())
            {
                var url = JsonHelpers.GetStringProperty(art, "image");
                var type = JsonHelpers.GetIntProperty(art, "type");

                // Type 1 = banner, 2 = poster, 3 = background, 7 = icon
                if (!string.IsNullOrEmpty(url) && (type == 2 || type == 3))
                {
                    images.Add(url);

                    // Limit to first few images
                    if (images.Count >= 5)
                    {
                        break;
                    }
                }
            }
        }

        // Fallback to image field
        if (images.Count == 0)
        {
            var imageUrl = JsonHelpers.GetStringProperty(element, "image");
            if (!string.IsNullOrEmpty(imageUrl))
            {
                images.Add($"https://artworks.thetvdb.com{imageUrl}");
            }
        }

        return images;
    }
}
