// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Core.ImportLists.Exceptions;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists.Trakt;

public class TraktImportList : ImportListBase<TraktSettings>
{
    private readonly IHttpClientFactory _httpClientFactory;

    public TraktImportList(ILogger<TraktImportList> logger, IHttpClientFactory httpClientFactory)
        : base(logger)
    {
        _httpClientFactory = httpClientFactory;
    }

    public override string Name => "Trakt";
    public override ImportListType ListType => ImportListType.Trakt;
    public override TimeSpan MinRefreshInterval => TimeSpan.FromHours(1);
    public override bool Enabled => !string.IsNullOrEmpty(Settings.ClientId);
    public override bool EnableAuto => Settings.HasValidToken;

    public override async Task<ImportListFetchResult> FetchAsync(CancellationToken cancellationToken = default)
    {
        var result = new ImportListFetchResult();
        var items = new List<ImportListItem>();

        try
        {
            if (!Settings.HasValidToken)
            {
                Logger.LogWarning("Trakt access token is missing or expired. Re-authorize via device code flow.");
                result.AnyFailure = true;
                return result;
            }

            var client = CreateAuthenticatedClient();

            if (Settings.ImportWatchlist)
            {
                var watchlist = await FetchWatchlistAsync(client, cancellationToken).ConfigureAwait(false);
                items.AddRange(watchlist);
            }

            if (Settings.ImportCollection)
            {
                var collection = await FetchCollectionAsync(client, cancellationToken).ConfigureAwait(false);
                items.AddRange(collection);
            }

            if (Settings.ImportWatchHistory)
            {
                var history = await FetchWatchHistoryAsync(client, cancellationToken).ConfigureAwait(false);
                items.AddRange(history);
            }

            if (Settings.ImportRatings)
            {
                var ratings = await FetchRatingsAsync(client, cancellationToken).ConfigureAwait(false);
                items.AddRange(ratings);
            }

            // Deduplicate by TMDB/TVDB ID
            result.Items = DeduplicateItems(items);
            result.SyncedLists = 1;

            Logger.LogInformation("Trakt import fetched {Count} unique items", result.Items.Count);
        }
        catch (HttpRequestException ex)
        {
            Logger.LogError(ex, "Failed to fetch from Trakt API");
            result.AnyFailure = true;
        }

        return result;
    }

    #region OAuth Device Code Flow

    /// <summary>
    /// Step 1: Request a device code from Trakt. Returns a code for the user to enter at trakt.tv/activate.
    /// </summary>
    public async Task<TraktDeviceCode> RequestDeviceCodeAsync(CancellationToken cancellationToken = default)
    {
        var client = _httpClientFactory.CreateClient();
        client.DefaultRequestHeaders.Add("trakt-api-version", "2");

        var content = new StringContent(
            JsonSerializer.Serialize(new { client_id = Settings.ClientId }),
            Encoding.UTF8,
            "application/json");

        var response = await client.PostAsync(
            $"{Settings.BaseUrl}/oauth/device/code",
            content,
            cancellationToken).ConfigureAwait(false);

        response.EnsureSuccessStatusCode();
        var json = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);
        return JsonSerializer.Deserialize<TraktDeviceCode>(json)!;
    }

    /// <summary>
    /// Step 2: Poll for user authorization. Call repeatedly at the interval specified in the device code response.
    /// Returns token response when authorized, null when still pending.
    /// </summary>
    public async Task<TraktTokenResponse?> PollForAuthorizationAsync(string deviceCode, CancellationToken cancellationToken = default)
    {
        var client = _httpClientFactory.CreateClient();
        client.DefaultRequestHeaders.Add("trakt-api-version", "2");

        var content = new StringContent(
            JsonSerializer.Serialize(new
            {
                code = deviceCode,
                client_id = Settings.ClientId,
                client_secret = Settings.ClientSecret
            }),
            Encoding.UTF8,
            "application/json");

        var response = await client.PostAsync(
            $"{Settings.BaseUrl}/oauth/device/token",
            content,
            cancellationToken).ConfigureAwait(false);

        if (response.StatusCode == System.Net.HttpStatusCode.BadRequest)
        {
            // 400 = authorization pending, user hasn't entered code yet
            return null;
        }

        if (response.StatusCode == System.Net.HttpStatusCode.Gone)
        {
            throw new ImportListException("Trakt device code expired. Please restart authorization.");
        }

        response.EnsureSuccessStatusCode();
        var json = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);
        return JsonSerializer.Deserialize<TraktTokenResponse>(json)!;
    }

    /// <summary>
    /// Refresh an expired access token using the refresh token.
    /// </summary>
    public async Task<TraktTokenResponse> RefreshAccessTokenAsync(CancellationToken cancellationToken = default)
    {
        var client = _httpClientFactory.CreateClient();
        client.DefaultRequestHeaders.Add("trakt-api-version", "2");

        var content = new StringContent(
            JsonSerializer.Serialize(new
            {
                refresh_token = Settings.RefreshToken,
                client_id = Settings.ClientId,
                client_secret = Settings.ClientSecret,
                redirect_uri = "urn:ietf:wg:oauth:2.0:oob",
                grant_type = "refresh_token"
            }),
            Encoding.UTF8,
            "application/json");

        var response = await client.PostAsync(
            $"{Settings.BaseUrl}/oauth/token",
            content,
            cancellationToken).ConfigureAwait(false);

        response.EnsureSuccessStatusCode();
        var json = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);
        return JsonSerializer.Deserialize<TraktTokenResponse>(json)!;
    }

    #endregion

    #region Fetch Methods

    private async Task<List<ImportListItem>> FetchWatchlistAsync(HttpClient client, CancellationToken ct)
    {
        var response = await client.GetAsync(
            $"{Settings.BaseUrl}/users/me/watchlist/movies,shows",
            ct).ConfigureAwait(false);

        response.EnsureSuccessStatusCode();
        var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        var items = JsonSerializer.Deserialize<List<TraktWatchlistItem>>(json) ?? new();

        return items.Select(i => MapWatchlistItem(i)).Where(i => i != null).Select(i => i!).ToList();
    }

    private async Task<List<ImportListItem>> FetchCollectionAsync(HttpClient client, CancellationToken ct)
    {
        var items = new List<ImportListItem>();

        // Movies
        var movieResponse = await client.GetAsync(
            $"{Settings.BaseUrl}/users/me/collection/movies",
            ct).ConfigureAwait(false);
        movieResponse.EnsureSuccessStatusCode();
        var movieJson = await movieResponse.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        var movieItems = JsonSerializer.Deserialize<List<TraktCollectionItem>>(movieJson) ?? new();
        items.AddRange(movieItems.Where(i => i.Movie != null).Select(i => MapMovie(i.Movie!)));

        // Shows
        var showResponse = await client.GetAsync(
            $"{Settings.BaseUrl}/users/me/collection/shows",
            ct).ConfigureAwait(false);
        showResponse.EnsureSuccessStatusCode();
        var showJson = await showResponse.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        var showItems = JsonSerializer.Deserialize<List<TraktCollectionItem>>(showJson) ?? new();
        items.AddRange(showItems.Where(i => i.Show != null).Select(i => MapShow(i.Show!)));

        return items;
    }

    private async Task<List<ImportListItem>> FetchWatchHistoryAsync(HttpClient client, CancellationToken ct)
    {
        var url = $"{Settings.BaseUrl}/users/me/history/movies,shows?limit=500";

        // Incremental sync: only fetch since last sync
        if (Settings.LastSyncedAt.HasValue)
        {
            url += $"&start_at={Settings.LastSyncedAt.Value:yyyy-MM-ddTHH:mm:ss.fffZ}";
        }

        var response = await client.GetAsync(url, ct).ConfigureAwait(false);
        response.EnsureSuccessStatusCode();
        var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        var items = JsonSerializer.Deserialize<List<TraktHistoryItem>>(json) ?? new();

        return items.Select(i => MapHistoryItem(i)).Where(i => i != null).Select(i => i!).ToList();
    }

    private async Task<List<ImportListItem>> FetchRatingsAsync(HttpClient client, CancellationToken ct)
    {
        var response = await client.GetAsync(
            $"{Settings.BaseUrl}/users/me/ratings/movies,shows",
            ct).ConfigureAwait(false);

        response.EnsureSuccessStatusCode();
        var json = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        var items = JsonSerializer.Deserialize<List<TraktRatingItem>>(json) ?? new();

        return items.Select(i => MapRatingItem(i)).Where(i => i != null).Select(i => i!).ToList();
    }

    #endregion

    #region Mapping

    private ImportListItem? MapWatchlistItem(TraktWatchlistItem item)
    {
        if (item.Movie != null) return MapMovie(item.Movie);
        if (item.Show != null) return MapShow(item.Show);
        return null;
    }

    private ImportListItem? MapHistoryItem(TraktHistoryItem item)
    {
        if (item.Movie != null) return MapMovie(item.Movie);
        if (item.Show != null) return MapShow(item.Show);
        return null;
    }

    private ImportListItem? MapRatingItem(TraktRatingItem item)
    {
        if (item.Movie != null) return MapMovie(item.Movie);
        if (item.Show != null) return MapShow(item.Show);
        return null;
    }

    private ImportListItem MapMovie(TraktMovie movie)
    {
        return new ImportListItem
        {
            ListId = Definition.Id,
            MediaType = MediaType.Movie,
            Title = movie.Title,
            Year = movie.Year ?? 0,
            TmdbId = movie.Ids.Tmdb ?? 0,
            ImdbId = movie.Ids.Imdb
        };
    }

    private ImportListItem MapShow(TraktShow show)
    {
        return new ImportListItem
        {
            ListId = Definition.Id,
            MediaType = MediaType.TV,
            Title = show.Title,
            Year = show.Year ?? 0,
            TmdbId = show.Ids.Tmdb ?? 0,
            TvdbId = show.Ids.Tvdb ?? 0,
            ImdbId = show.Ids.Imdb
        };
    }

    #endregion

    private HttpClient CreateAuthenticatedClient()
    {
        var client = _httpClientFactory.CreateClient();
        client.DefaultRequestHeaders.Add("trakt-api-version", "2");
        client.DefaultRequestHeaders.Add("trakt-api-key", Settings.ClientId);
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", Settings.AccessToken);
        return client;
    }

    private static List<ImportListItem> DeduplicateItems(List<ImportListItem> items)
    {
        var seen = new HashSet<string>();
        var result = new List<ImportListItem>();

        foreach (var item in items)
        {
            var key = item.MediaType == MediaType.Movie
                ? $"movie:{item.TmdbId}"
                : $"show:{item.TvdbId}";

            if (key != "movie:0" && key != "show:0" && !seen.Add(key))
                continue;

            result.Add(item);
        }

        return result;
    }
}
