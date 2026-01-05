// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http;
using System.Text.Json;
using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.Logging;
using Mouseion.Common.Extensions;
using Mouseion.Common.Http;
using Mouseion.Core.Music;

namespace Mouseion.Core.MetadataSource;

public class MusicBrainzInfoProxy : IProvideMusicInfo
{
    private const string BaseUrl = "https://musicbrainz.org/ws/2";
    private const string UserAgent = "Mouseion/1.0 (https://github.com/forkwright/mouseion)";

    private readonly IHttpClient _httpClient;
    private readonly IMemoryCache _cache;
    private readonly ILogger<MusicBrainzInfoProxy> _logger;

    public MusicBrainzInfoProxy(IHttpClient httpClient, IMemoryCache cache, ILogger<MusicBrainzInfoProxy> logger)
    {
        _httpClient = httpClient;
        _cache = cache;
        _logger = logger;
    }

    public async Task<Artist?> GetArtistByMusicBrainzIdAsync(string mbid, CancellationToken ct = default)
    {
        var cacheKey = $"artist_mbid_{mbid}";

        if (_cache.TryGetValue(cacheKey, out Artist? cached))
        {
            _logger.LogDebug("Cache hit for artist MBID: {Mbid}", mbid);
            return cached;
        }

        try
        {
            _logger.LogDebug("Fetching artist by MBID: {Mbid}", mbid);

            var url = $"{BaseUrl}/artist/{mbid}?fmt=json&inc=genres+url-rels+tags";
            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("MusicBrainz returned {StatusCode} for artist {Mbid}", response.StatusCode, mbid);
                return null;
            }

            var result = ParseArtist(response.Content, mbid);
            if (result != null)
            {
                _cache.Set(cacheKey, result, TimeSpan.FromMinutes(15));
            }

            return result;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error fetching artist by MBID: {Mbid}", mbid);
            return null;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse response for artist MBID: {Mbid}", mbid);
            return null;
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: artist by MBID {Mbid}", mbid);
            return null;
        }
    }

    public async Task<Album?> GetAlbumByMusicBrainzIdAsync(string mbid, CancellationToken ct = default)
    {
        var cacheKey = $"album_mbid_{mbid}";

        if (_cache.TryGetValue(cacheKey, out Album? cached))
        {
            _logger.LogDebug("Cache hit for album MBID: {Mbid}", mbid);
            return cached;
        }

        try
        {
            _logger.LogDebug("Fetching album by MBID: {Mbid}", mbid);

            var url = $"{BaseUrl}/release/{mbid}?fmt=json&inc=artists+genres+labels+recordings+tags";
            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("MusicBrainz returned {StatusCode} for album {Mbid}", response.StatusCode, mbid);
                return null;
            }

            var result = ParseAlbum(response.Content, mbid);
            if (result != null)
            {
                _cache.Set(cacheKey, result, TimeSpan.FromMinutes(15));
            }

            return result;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error fetching album by MBID: {Mbid}", mbid);
            return null;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse response for album MBID: {Mbid}", mbid);
            return null;
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: album by MBID {Mbid}", mbid);
            return null;
        }
    }

    public async Task<List<Artist>> SearchArtistsByNameAsync(string name, CancellationToken ct = default)
    {
        var cacheKey = $"artist_search_{name.ToLowerInvariant()}";

        if (_cache.TryGetValue(cacheKey, out List<Artist>? cached))
        {
            _logger.LogDebug("Cache hit for artist search: {Name}", name.SanitizeForLog());
            return cached ?? new List<Artist>();
        }

        try
        {
            _logger.LogDebug("Searching artists by name: {Name}", name.SanitizeForLog());

            var url = $"{BaseUrl}/artist?query={Uri.EscapeDataString(name)}&fmt=json&limit=25";
            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("MusicBrainz search returned {StatusCode}", response.StatusCode);
                return new List<Artist>();
            }

            var result = ParseArtistSearchResults(response.Content);
            _cache.Set(cacheKey, result, TimeSpan.FromMinutes(15));

            return result;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error searching artists by name: {Name}", name.SanitizeForLog());
            return new List<Artist>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse search results for artist name: {Name}", name.SanitizeForLog());
            return new List<Artist>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: artist name search {Name}", name.SanitizeForLog());
            return new List<Artist>();
        }
    }

    public async Task<List<Album>> SearchAlbumsByTitleAsync(string title, CancellationToken ct = default)
    {
        var cacheKey = $"album_search_{title.ToLowerInvariant()}";

        if (_cache.TryGetValue(cacheKey, out List<Album>? cached))
        {
            _logger.LogDebug("Cache hit for album search: {Title}", title.SanitizeForLog());
            return cached ?? new List<Album>();
        }

        try
        {
            _logger.LogDebug("Searching albums by title: {Title}", title.SanitizeForLog());

            var url = $"{BaseUrl}/release?query={Uri.EscapeDataString(title)}&fmt=json&limit=25";
            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("MusicBrainz search returned {StatusCode}", response.StatusCode);
                return new List<Album>();
            }

            var result = ParseAlbumSearchResults(response.Content);
            _cache.Set(cacheKey, result, TimeSpan.FromMinutes(15));

            return result;
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error searching albums by title: {Title}", title.SanitizeForLog());
            return new List<Album>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse search results for album title: {Title}", title.SanitizeForLog());
            return new List<Album>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: album title search {Title}", title.SanitizeForLog());
            return new List<Album>();
        }
    }

    public async Task<List<Album>> GetAlbumsByArtistAsync(string artistMbid, CancellationToken ct = default)
    {
        try
        {
            _logger.LogDebug("Fetching albums for artist: {ArtistMbid}", artistMbid);

            var url = $"{BaseUrl}/release?artist={artistMbid}&fmt=json&limit=100";
            var request = new HttpRequestBuilder(url)
                .SetHeader("User-Agent", UserAgent)
                .Build();

            var response = await _httpClient.GetAsync(request).ConfigureAwait(false);
            if (response.StatusCode != System.Net.HttpStatusCode.OK)
            {
                _logger.LogWarning("MusicBrainz returned {StatusCode}", response.StatusCode);
                return new List<Album>();
            }

            return ParseAlbumSearchResults(response.Content);
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError(ex, "Network error fetching albums for artist: {ArtistMbid}", artistMbid);
            return new List<Album>();
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Failed to parse response for albums by artist: {ArtistMbid}", artistMbid);
            return new List<Album>();
        }
        catch (TaskCanceledException ex)
        {
            _logger.LogWarning(ex, "Request timed out or was cancelled: albums for artist {ArtistMbid}", artistMbid);
            return new List<Album>();
        }
    }

    public async Task<List<Album>> GetTrendingAlbumsAsync(CancellationToken ct = default)
    {
        _logger.LogDebug("GetTrendingAlbums not supported by MusicBrainz");
        return await Task.FromResult(new List<Album>()).ConfigureAwait(false);
    }

    public async Task<List<Album>> GetPopularAlbumsAsync(CancellationToken ct = default)
    {
        _logger.LogDebug("GetPopularAlbums not supported by MusicBrainz");
        return await Task.FromResult(new List<Album>()).ConfigureAwait(false);
    }

    private Artist? ParseArtist(string json, string mbid)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            var lifeSpanJson = root.TryGetProperty("life-span", out var lifeSpanElement) ? lifeSpanElement.GetRawText() : null;

            var artist = new Artist
            {
                Name = root.GetProperty("name").GetString() ?? "Unknown",
                SortName = JsonHelpers.GetStringProperty(root, "sort-name"),
                MusicBrainzId = mbid,
                ForeignArtistId = mbid,
                Description = JsonHelpers.GetStringProperty(root, "disambiguation"),
                ArtistType = JsonHelpers.GetStringProperty(root, "type"),
                Country = JsonHelpers.GetStringProperty(root, "country"),
                Genres = GetStringArrayProperty(root, "genres"),
                BeginDate = ParseDate(lifeSpanJson, "begin"),
                EndDate = ParseDate(lifeSpanJson, "end")
            };

            return artist;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing MusicBrainz artist JSON");
            return null;
        }
    }

    private Album? ParseAlbum(string json, string mbid)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            var album = new Album
            {
                Title = root.GetProperty("title").GetString() ?? "Unknown",
                MusicBrainzId = mbid,
                ForeignAlbumId = mbid,
                ReleaseDate = JsonHelpers.ParseReleaseDate(JsonHelpers.GetStringProperty(root, "date")),
                AlbumType = JsonHelpers.GetStringProperty(root, "packaging"),
                Genres = GetStringArrayProperty(root, "genres"),
                TrackCount = JsonHelpers.GetIntProperty(root, "track-count")
            };

            return album;
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing MusicBrainz album JSON");
            return null;
        }
    }

    private List<Artist> ParseArtistSearchResults(string json)
    {
        var artists = new List<Artist>();

        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            if (root.TryGetProperty("artists", out var artistsArray))
            {
                foreach (var item in artistsArray.EnumerateArray())
                {
                    var mbid = JsonHelpers.GetStringProperty(item, "id");
                    if (string.IsNullOrWhiteSpace(mbid))
                    {
                        continue;
                    }

                    var artist = new Artist
                    {
                        Name = item.GetProperty("name").GetString() ?? "Unknown",
                        SortName = JsonHelpers.GetStringProperty(item, "sort-name"),
                        MusicBrainzId = mbid,
                        ForeignArtistId = mbid,
                        ArtistType = JsonHelpers.GetStringProperty(item, "type"),
                        Country = JsonHelpers.GetStringProperty(item, "country")
                    };

                    artists.Add(artist);
                }
            }
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing MusicBrainz artist search results");
        }

        return artists;
    }

    private List<Album> ParseAlbumSearchResults(string json)
    {
        var albums = new List<Album>();

        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            if (root.TryGetProperty("releases", out var releasesArray))
            {
                foreach (var item in releasesArray.EnumerateArray())
                {
                    var mbid = JsonHelpers.GetStringProperty(item, "id");
                    if (string.IsNullOrWhiteSpace(mbid))
                    {
                        continue;
                    }

                    var album = new Album
                    {
                        Title = item.GetProperty("title").GetString() ?? "Unknown",
                        MusicBrainzId = mbid,
                        ForeignAlbumId = mbid,
                        ReleaseDate = JsonHelpers.ParseReleaseDate(JsonHelpers.GetStringProperty(item, "date")),
                        TrackCount = JsonHelpers.GetIntProperty(item, "track-count")
                    };

                    albums.Add(album);
                }
            }
        }
        catch (JsonException ex)
        {
            _logger.LogError(ex, "Error parsing MusicBrainz album search results");
        }

        return albums;
    }

    private static List<string> GetStringArrayProperty(JsonElement element, string propertyName)
    {
        var result = new List<string>();

        if (element.TryGetProperty(propertyName, out var property) && property.ValueKind == JsonValueKind.Array)
        {
            foreach (var item in property.EnumerateArray())
            {
                if (item.TryGetProperty("name", out var nameElem) && nameElem.ValueKind == JsonValueKind.String)
                {
                    var value = nameElem.GetString();
                    if (!string.IsNullOrWhiteSpace(value))
                    {
                        result.Add(value);
                    }
                }
            }
        }

        return result;
    }

    private static DateTime? ParseDate(string? lifeSpan, string key)
    {
        if (string.IsNullOrWhiteSpace(lifeSpan))
        {
            return null;
        }

        try
        {
            using var doc = JsonDocument.Parse(lifeSpan);
            var dateStr = JsonHelpers.GetStringProperty(doc.RootElement, key);
            return JsonHelpers.ParseReleaseDate(dateStr);
        }
        catch
        {
            return null;
        }
    }
}
