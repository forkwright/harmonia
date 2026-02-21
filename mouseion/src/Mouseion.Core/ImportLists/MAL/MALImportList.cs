// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net.Http;
using System.Net.Http.Headers;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.ImportLists.MAL;

/// <summary>
/// MyAnimeList import list. Imports anime and manga lists via MAL API v2.
/// OAuth 2.0 authorization code flow with PKCE.
/// </summary>
public class MALImportList : ImportListBase<MALSettings>
{
    private readonly IHttpClientFactory _httpClientFactory;

    private const string AnimeFields = "list_status{status,score,num_episodes_watched,is_rewatching,updated_at,start_date,finish_date}," +
                                       "id,title,main_picture,start_date,media_type,num_episodes";
    private const string MangaFields = "list_status{status,score,num_chapters_read,num_volumes_read,is_rereading,updated_at,start_date,finish_date}," +
                                       "id,title,main_picture,start_date,media_type,num_chapters,num_volumes";

    public MALImportList(ILogger<MALImportList> logger, IHttpClientFactory httpClientFactory)
        : base(logger)
    {
        _httpClientFactory = httpClientFactory;
    }

    public override string Name => "MyAnimeList";
    public override ImportListType ListType => ImportListType.MAL;
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
                Logger.LogWarning("MAL access token is missing or expired. Re-authorize.");
                result.AnyFailure = true;
                return result;
            }

            var client = CreateAuthenticatedClient();

            if (Settings.ImportAnimeList)
            {
                var animeItems = await FetchAnimeListAsync(client, cancellationToken);
                items.AddRange(animeItems);
            }

            if (Settings.ImportMangaList)
            {
                var mangaItems = await FetchMangaListAsync(client, cancellationToken);
                items.AddRange(mangaItems);
            }

            // Deduplicate by MAL ID
            var uniqueItems = items
                .GroupBy(i => (i.MalId, i.MediaType))
                .Select(g => g.First())
                .ToList();

            result.Items = CleanupListItems(uniqueItems);
            result.SyncedLists = (Settings.ImportAnimeList ? 1 : 0) + (Settings.ImportMangaList ? 1 : 0);
        }
        catch (HttpRequestException ex)
        {
            Logger.LogError(ex, "Failed to fetch MAL lists");
            result.AnyFailure = true;
        }

        return result;
    }

    private async Task<List<ImportListItem>> FetchAnimeListAsync(HttpClient client, CancellationToken cancellationToken)
    {
        var items = new List<ImportListItem>();
        var url = $"{Settings.BaseUrl}/users/@me/animelist?fields={AnimeFields}&limit=100&nsfw=true";

        while (!string.IsNullOrEmpty(url))
        {
            var response = await client.GetStringAsync(url, cancellationToken);
            var page = JsonSerializer.Deserialize<MALPagedResponse<MALAnimeListItem>>(response);

            if (page?.Data == null) break;

            foreach (var entry in page.Data)
            {
                if (ShouldFilter(entry.ListStatus?.Status)) continue;

                items.Add(new ImportListItem
                {
                    MediaType = MediaType.Movie, // Anime maps to video content
                    Title = entry.Node.Title,
                    MalId = entry.Node.Id,
                    Year = ParseYear(entry.Node.StartDate),
                    UserRating = entry.ListStatus?.Score > 0 ? entry.ListStatus.Score : null,
                    WatchedAt = ParseDate(entry.ListStatus?.FinishDate),
                    ImportSource = "MAL"
                });
            }

            url = page.Paging?.Next;
        }

        Logger.LogInformation("Fetched {Count} anime from MAL", items.Count);
        return items;
    }

    private async Task<List<ImportListItem>> FetchMangaListAsync(HttpClient client, CancellationToken cancellationToken)
    {
        var items = new List<ImportListItem>();
        var url = $"{Settings.BaseUrl}/users/@me/mangalist?fields={MangaFields}&limit=100&nsfw=true";

        while (!string.IsNullOrEmpty(url))
        {
            var response = await client.GetStringAsync(url, cancellationToken);
            var page = JsonSerializer.Deserialize<MALPagedResponse<MALMangaListItem>>(response);

            if (page?.Data == null) break;

            foreach (var entry in page.Data)
            {
                if (ShouldFilter(entry.ListStatus?.Status)) continue;

                items.Add(new ImportListItem
                {
                    MediaType = MediaType.Manga,
                    Title = entry.Node.Title,
                    MalId = entry.Node.Id,
                    Year = ParseYear(entry.Node.StartDate),
                    UserRating = entry.ListStatus?.Score > 0 ? entry.ListStatus.Score : null,
                    WatchedAt = ParseDate(entry.ListStatus?.FinishDate),
                    ImportSource = "MAL"
                });
            }

            url = page.Paging?.Next;
        }

        Logger.LogInformation("Fetched {Count} manga from MAL", items.Count);
        return items;
    }

    private bool ShouldFilter(string? status)
    {
        if (string.IsNullOrEmpty(status) || Settings.StatusFilter.Count == 0)
        {
            return false;
        }

        return !Settings.StatusFilter.Contains(status, StringComparer.OrdinalIgnoreCase);
    }

    private HttpClient CreateAuthenticatedClient()
    {
        var client = _httpClientFactory.CreateClient("MAL");
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", Settings.AccessToken);
        return client;
    }

    private static int ParseYear(string? dateStr)
    {
        if (string.IsNullOrEmpty(dateStr)) return 0;

        // MAL dates are "YYYY-MM-DD" or "YYYY"
        if (dateStr.Length >= 4 && int.TryParse(dateStr[..4], out var year))
        {
            return year;
        }

        return 0;
    }

    private static DateTime? ParseDate(string? dateStr)
    {
        if (string.IsNullOrEmpty(dateStr)) return null;
        return DateTime.TryParse(dateStr, out var date) ? date : null;
    }
}
