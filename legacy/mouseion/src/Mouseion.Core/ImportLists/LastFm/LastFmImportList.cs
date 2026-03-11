// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists.LastFm;

/// <summary>
/// Imports music data from Last.fm via their public API.
/// Last.fm API requires only an API key for read access (no OAuth).
/// Rate limit: 5 requests/second per API key.
/// Docs: https://www.last.fm/api
/// </summary>
public class LastFmImportList : ImportListBase<LastFmSettings>
{
    private readonly IHttpClientFactory _httpClientFactory;
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true
    };

    public LastFmImportList(ILogger<LastFmImportList> logger, IHttpClientFactory httpClientFactory)
        : base(logger)
    {
        _httpClientFactory = httpClientFactory;
    }

    public override string Name => "Last.fm";
    public override ImportListType ListType => ImportListType.LastFm;
    public override TimeSpan MinRefreshInterval => TimeSpan.FromHours(6);
    public override bool Enabled => Settings.IsConfigured;
    public override bool EnableAuto => Settings.IsConfigured;

    public override async Task<ImportListFetchResult> FetchAsync(CancellationToken cancellationToken = default)
    {
        var result = new ImportListFetchResult();
        var allItems = new List<ImportListItem>();

        try
        {
            if (!Settings.IsConfigured)
            {
                Logger.LogWarning("Last.fm API key or username not configured");
                result.AnyFailure = true;
                return result;
            }

            var client = _httpClientFactory.CreateClient();
            var sourceCount = 0;

            if (Settings.ImportTopAlbums)
            {
                var albums = await FetchTopAlbumsAsync(client, cancellationToken).ConfigureAwait(false);
                allItems.AddRange(albums);
                sourceCount++;
            }

            if (Settings.ImportTopArtists)
            {
                var artists = await FetchTopArtistsAsync(client, cancellationToken).ConfigureAwait(false);
                allItems.AddRange(artists);
                sourceCount++;
            }

            if (Settings.ImportLovedTracks)
            {
                var loved = await FetchLovedTracksAsync(client, cancellationToken).ConfigureAwait(false);
                allItems.AddRange(loved);
                sourceCount++;
            }

            if (Settings.ImportRecentTracks)
            {
                var recent = await FetchRecentTracksAsync(client, cancellationToken).ConfigureAwait(false);
                allItems.AddRange(recent);
                sourceCount++;
            }

            // Deduplicate by MusicBrainz ID or artist+album
            result.Items = DeduplicateItems(allItems);
            result.SyncedLists = sourceCount;

            Logger.LogInformation(
                "Last.fm import fetched {Count} unique items from {SourceCount} sources for user {Username}",
                result.Items.Count, sourceCount, Settings.Username);
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            Logger.LogError(ex, "Failed to complete Last.fm import");
            result.AnyFailure = true;
        }

        return result;
    }

    #region Fetch Methods

    private async Task<List<ImportListItem>> FetchTopAlbumsAsync(HttpClient client, CancellationToken ct)
    {
        var items = new List<ImportListItem>();
        var page = 1;
        var totalPages = 1;

        while (page <= totalPages && items.Count < Settings.MaxItemsPerCategory)
        {
            var url = BuildUrl("user.getTopAlbums", new Dictionary<string, string>
            {
                ["user"] = Settings.Username,
                ["period"] = Settings.TimePeriod,
                ["limit"] = "50",
                ["page"] = page.ToString()
            });

            var response = await client.GetAsync(url, ct).ConfigureAwait(false);
            response.EnsureSuccessStatusCode();
            var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
            var data = JsonSerializer.Deserialize<LastFmTopAlbumsResponse>(json, JsonOptions);

            if (data?.TopAlbums.Albums == null || data.TopAlbums.Albums.Count == 0)
                break;

            totalPages = int.TryParse(data.TopAlbums.Attr.TotalPages, out var tp) ? tp : 1;

            foreach (var album in data.TopAlbums.Albums)
            {
                var item = new ImportListItem
                {
                    ListId = Definition.Id,
                    MediaType = MediaType.Music,
                    Title = album.Name,
                    Artist = album.Artist.Name,
                    ImportSource = "lastfm:top-albums"
                };

                if (TryParseMbid(album.Mbid, out var mbid))
                {
                    item.MusicBrainzId = mbid;
                }

                items.Add(item);
            }

            page++;
            await RateLimitDelay(ct).ConfigureAwait(false);
        }

        return items;
    }

    private async Task<List<ImportListItem>> FetchTopArtistsAsync(HttpClient client, CancellationToken ct)
    {
        var items = new List<ImportListItem>();
        var page = 1;
        var totalPages = 1;

        while (page <= totalPages && items.Count < Settings.MaxItemsPerCategory)
        {
            var url = BuildUrl("user.getTopArtists", new Dictionary<string, string>
            {
                ["user"] = Settings.Username,
                ["period"] = Settings.TimePeriod,
                ["limit"] = "50",
                ["page"] = page.ToString()
            });

            var response = await client.GetAsync(url, ct).ConfigureAwait(false);
            response.EnsureSuccessStatusCode();
            var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
            var data = JsonSerializer.Deserialize<LastFmTopArtistsResponse>(json, JsonOptions);

            if (data?.TopArtists.Artists == null || data.TopArtists.Artists.Count == 0)
                break;

            totalPages = int.TryParse(data.TopArtists.Attr.TotalPages, out var tp) ? tp : 1;

            foreach (var artist in data.TopArtists.Artists)
            {
                var item = new ImportListItem
                {
                    ListId = Definition.Id,
                    MediaType = MediaType.Music,
                    Title = artist.Name, // Artist name as title for discovery
                    Artist = artist.Name,
                    ImportSource = "lastfm:top-artists"
                };

                if (TryParseMbid(artist.Mbid, out var mbid))
                {
                    item.MusicBrainzId = mbid;
                }

                items.Add(item);
            }

            page++;
            await RateLimitDelay(ct).ConfigureAwait(false);
        }

        return items;
    }

    private async Task<List<ImportListItem>> FetchLovedTracksAsync(HttpClient client, CancellationToken ct)
    {
        var items = new List<ImportListItem>();
        var page = 1;
        var totalPages = 1;

        while (page <= totalPages && items.Count < Settings.MaxItemsPerCategory)
        {
            var url = BuildUrl("user.getLovedTracks", new Dictionary<string, string>
            {
                ["user"] = Settings.Username,
                ["limit"] = "50",
                ["page"] = page.ToString()
            });

            var response = await client.GetAsync(url, ct).ConfigureAwait(false);
            response.EnsureSuccessStatusCode();
            var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
            var data = JsonSerializer.Deserialize<LastFmLovedTracksResponse>(json, JsonOptions);

            if (data?.LovedTracks.Tracks == null || data.LovedTracks.Tracks.Count == 0)
                break;

            totalPages = int.TryParse(data.LovedTracks.Attr.TotalPages, out var tp) ? tp : 1;

            foreach (var track in data.LovedTracks.Tracks)
            {
                var item = new ImportListItem
                {
                    ListId = Definition.Id,
                    MediaType = MediaType.Music,
                    Title = track.Name,
                    Artist = track.Artist.Name,
                    UserRating = 10, // Loved = max rating
                    ImportSource = "lastfm:loved"
                };

                if (TryParseMbid(track.Mbid, out var mbid))
                {
                    item.MusicBrainzId = mbid;
                }

                if (track.Date != null && long.TryParse(track.Date.Uts, out var uts) && uts > 0)
                {
                    item.WatchedAt = DateTimeOffset.FromUnixTimeSeconds(uts).UtcDateTime;
                }

                items.Add(item);
            }

            page++;
            await RateLimitDelay(ct).ConfigureAwait(false);
        }

        return items;
    }

    private async Task<List<ImportListItem>> FetchRecentTracksAsync(HttpClient client, CancellationToken ct)
    {
        var items = new List<ImportListItem>();
        var page = 1;
        var totalPages = 1;
        const int maxPages = 200; // Safety cap: 200 * 200 = 40,000 scrobbles

        var parameters = new Dictionary<string, string>
        {
            ["user"] = Settings.Username,
            ["limit"] = "200",
            ["page"] = "1",
            ["extended"] = "0"
        };

        // Incremental sync: only fetch since last sync
        if (Settings.LastSyncedAt.HasValue)
        {
            var unixTime = new DateTimeOffset(Settings.LastSyncedAt.Value).ToUnixTimeSeconds();
            parameters["from"] = unixTime.ToString();
        }

        while (page <= totalPages && page <= maxPages)
        {
            parameters["page"] = page.ToString();
            var url = BuildUrl("user.getRecentTracks", parameters);

            var response = await client.GetAsync(url, ct).ConfigureAwait(false);
            response.EnsureSuccessStatusCode();
            var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
            var data = JsonSerializer.Deserialize<LastFmRecentTracksResponse>(json, JsonOptions);

            if (data?.RecentTracks.Tracks == null || data.RecentTracks.Tracks.Count == 0)
                break;

            totalPages = int.TryParse(data.RecentTracks.Attr.TotalPages, out var tp) ? tp : 1;

            foreach (var track in data.RecentTracks.Tracks)
            {
                // Skip currently playing track (no timestamp)
                if (track.IsNowPlaying)
                    continue;

                var item = new ImportListItem
                {
                    ListId = Definition.Id,
                    MediaType = MediaType.Music,
                    Title = track.Name,
                    Artist = track.Artist.Name,
                    Album = track.Album.Name,
                    ImportSource = "lastfm:scrobble"
                };

                if (TryParseMbid(track.Mbid, out var mbid))
                {
                    item.MusicBrainzId = mbid;
                }
                else if (TryParseMbid(track.Album.Mbid, out var albumMbid))
                {
                    // Fall back to album MBID if track MBID is missing
                    item.MusicBrainzId = albumMbid;
                }

                if (track.Date != null && long.TryParse(track.Date.Uts, out var uts) && uts > 0)
                {
                    item.WatchedAt = DateTimeOffset.FromUnixTimeSeconds(uts).UtcDateTime;
                }

                items.Add(item);
            }

            page++;
            await RateLimitDelay(ct).ConfigureAwait(false);
        }

        return items;
    }

    #endregion

    #region Helpers

    private string BuildUrl(string method, Dictionary<string, string> parameters)
    {
        var queryString = string.Join("&", parameters.Select(p =>
            $"{Uri.EscapeDataString(p.Key)}={Uri.EscapeDataString(p.Value)}"));

        return $"{Settings.BaseUrl}?method={method}&api_key={Settings.ApiKey}&format=json&{queryString}";
    }

    private static bool TryParseMbid(string? mbid, out Guid result)
    {
        result = Guid.Empty;
        if (string.IsNullOrWhiteSpace(mbid))
            return false;

        return Guid.TryParse(mbid, out result) && result != Guid.Empty;
    }

    private static List<ImportListItem> DeduplicateItems(List<ImportListItem> items)
    {
        var seen = new HashSet<string>();
        var result = new List<ImportListItem>();

        foreach (var item in items)
        {
            // Prefer MusicBrainz ID for dedup, fall back to artist+title
            var key = item.MusicBrainzId != Guid.Empty
                ? $"mbid:{item.MusicBrainzId}"
                : $"text:{item.Artist?.ToLowerInvariant()}:{item.Title.ToLowerInvariant()}:{item.Album?.ToLowerInvariant()}";

            if (seen.Add(key))
            {
                result.Add(item);
            }
        }

        return result;
    }

    /// <summary>
    /// Last.fm rate limit: 5 requests/second. 200ms delay keeps us under.
    /// </summary>
    private static Task RateLimitDelay(CancellationToken ct) => Task.Delay(200, ct);

    #endregion
}
